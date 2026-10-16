//! The open files' language servers: told what the editor does, started on a file's first open.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use groove_types::{Caret, Config, Diagnostic, Error, ErrorKind, LanguageServer, WorktreeId};
use groove_workspace_service::{
    Client, Hooks, Language, ServerKey, Tell, Telling, launch, serving, start,
};

use crate::spawn::coalesced;
use crate::{AppState, Continuation, Deliver, Services, Spawner};

type Heard = Arc<Mutex<Vec<(PathBuf, Vec<Diagnostic>)>>>;

/// A file of `worktree` read into its buffer: its servers open it, and start if they must.
pub(super) fn opened(
    state: &mut AppState,
    spawner: &dyn Spawner,
    worktree: &WorktreeId,
    path: &str,
) {
    let Some((language, text)) = held(state, path) else {
        return;
    };
    let tell = Tell::Opened {
        path: path.to_string(),
        language: language.id(),
        text,
    };
    told(state, Some(spawner), worktree, language, tell);
}

/// The active buffer typed in: its servers get its whole new text.
pub(super) fn changed(state: &mut AppState, path: &str) {
    let (Some(worktree), Some((language, text))) =
        (state.workspace.worktree.clone(), held(state, path))
    else {
        return;
    };
    let tell = Tell::Changed {
        path: path.to_string(),
        text,
    };
    told(state, None, &worktree, language, tell);
}

pub(super) fn saved(state: &mut AppState, worktree: &WorktreeId, path: &str) {
    let language = Language::of(path);
    let tell = Tell::Saved {
        path: path.to_string(),
    };
    told(state, None, worktree, language, tell);
}

/// A tab about to close: its servers let the file go.
pub(super) fn closing(state: &mut AppState, path: &str) {
    let Some(worktree) = state.workspace.worktree.clone() else {
        return;
    };
    let language = held(state, path).map(|(language, _)| language);
    let tell = Tell::Closed {
        path: path.to_string(),
    };
    told(state, None, &worktree, language, tell);
}

/// What the servers of `path` say of the symbol at `at`; the answer lands while it is still asked.
pub(super) fn hover(
    state: &mut AppState,
    spawner: &dyn Spawner,
    path: String,
    (at, caret): (Caret, bool),
) {
    state.workspace.servers.asked(&path, at, caret);
    let Some((dir, clients)) = clients_of(state, &path) else {
        return;
    };
    let file = dir.join(&path);
    spawner.spawn(Box::pin(async move {
        let mut said = None;
        for client in clients {
            if let Ok(Some(text)) = client.hover(&file, at).await {
                said = Some(text);
                break;
            }
        }
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            state.workspace.servers.answered(&path, at, said);
        }) as Continuation
    }));
}

/// The selected worktree's directory, and the clients up for the language of its open `path`.
pub(super) fn clients_of(state: &AppState, path: &str) -> Option<(PathBuf, Vec<Arc<Client>>)> {
    let worktree = state.workspace.worktree.clone()?;
    let dir = state.session.find(&worktree).map(|(_, _, one)| one.dir())?;
    let open = state.workspace.buffer(path)?;
    let language = open.new.document().language()?;
    let names: Vec<String> = configured(state)
        .into_iter()
        .filter(|one| one.serves(language.id()))
        .map(|one| one.name)
        .collect();
    let clients = state.workspace.servers.clients(&worktree, &names);
    (!clients.is_empty()).then_some((dir, clients))
}

fn configured(state: &AppState) -> Vec<LanguageServer> {
    let config = state.config.config.as_ref();
    config.map_or_else(LanguageServer::defaults, Config::servers)
}

fn held(state: &AppState, path: &str) -> Option<(Language, String)> {
    let open = state.workspace.buffer(path)?;
    Some((open.new.document().language()?, open.new.text()))
}

fn told(
    state: &mut AppState,
    spawner: Option<&dyn Spawner>,
    worktree: &WorktreeId,
    language: impl Into<Option<Language>>,
    tell: Tell,
) {
    let Some(language) = language.into() else {
        return;
    };
    let Some(dir) = state.session.find(worktree).map(|(_, _, one)| one.dir()) else {
        return;
    };
    let servers = configured(state);
    for server in serving(&servers, language.id()) {
        let key = ServerKey {
            worktree: worktree.clone(),
            dir: dir.clone(),
            name: server.name.clone(),
        };
        let telling = state.workspace.servers.tell(&key, tell.clone());
        if let (Telling::Start, Some(spawner)) = (telling, spawner) {
            begin(spawner, key, server);
        }
    }
}

/// Starts `server`; what it was told while starting follows once it is up.
fn begin(spawner: &dyn Spawner, key: ServerKey, server: &LanguageServer) {
    let launch = launch(server, &key.dir);
    let hooks = hooks(spawner.sink(), key.clone());
    spawner.spawn(Box::pin(async move {
        let client = start(launch, hooks).await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            if let Err(e) = state.workspace.servers.started(&key, client) {
                state.failed(unstarted(&key.name, e));
            }
        }) as Continuation
    }));
}

/// Why a server did not start; one not installed says how to get it.
fn unstarted(name: &str, e: Error) -> Error {
    let said = match e.kind {
        ErrorKind::NotFound => format!(
            "{name} is not installed: {}. Install it, or set its program under `language_servers` in the config.",
            e.message
        ),
        _ => format!("{name} did not start: {}", e.message),
    };
    Error::new(e.kind, said)
}

/// Diagnostics batched to one continuation in flight; a close delivered as it comes.
fn hooks(sink: Arc<dyn Deliver>, key: ServerKey) -> Hooks {
    let heard: Heard = Arc::default();
    let queue = heard.clone();
    let (from, owner) = (key.clone(), key);
    let once = coalesced(sink.clone(), move || {
        let (key, heard) = (from.clone(), heard.clone());
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            for (path, found) in std::mem::take(&mut *lock(&heard)) {
                state.workspace.servers.diagnosed(&key, &path, found);
            }
        }) as Continuation
    });
    Hooks {
        on_diagnostics: Box::new(move |path, found| {
            lock(&queue).push((path, found));
            once();
        }),
        on_closed: Box::new(move || {
            sink.deliver(Box::new(
                move |state: &mut AppState, _: &Services, _: &dyn Spawner| down(state, &owner),
            ));
        }),
    }
}

fn down(state: &mut AppState, key: &ServerKey) {
    state.workspace.servers.closed(key);
}

fn lock<T>(held: &Mutex<T>) -> MutexGuard<'_, T> {
    held.lock().unwrap_or_else(PoisonError::into_inner)
}
