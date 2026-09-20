//! The `task` controller: one function per user action on the `task` service.

use groove_session_service::task_session;
use groove_task_service::{fetch, list, sources};
use groove_types::{SessionId, SessionKind, Task, TaskKey, Timestamp};

use crate::{AppState, Continuation, Services, Spawner, agent, session};

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// `task.load`: every task the configured sources hold.
    Load,
    /// `task.sync`: one task, read again from its source.
    Sync { key: TaskKey },
    /// `task.open`: the session that works this task, created or selected.
    Open { short_id: String },
}

impl Command {
    pub fn id(&self) -> &'static str {
        match self {
            Command::Load => "task.load",
            Command::Sync { .. } => "task.sync",
            Command::Open { .. } => "task.open",
        }
    }
}

pub fn dispatch(
    command: Command,
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
) {
    match command {
        Command::Load => load(state, spawner),
        Command::Sync { key } => sync(state, spawner, key),
        Command::Open { short_id } => open(state, services, spawner, &short_id),
    }
}

/// The session that works this task: the one it already has, or a new one with its
/// agent started.
pub fn open(state: &mut AppState, services: &Services, spawner: &dyn Spawner, short_id: &str) {
    let Some(task) = state.task.get(short_id).cloned() else {
        return;
    };
    if let Some(id) = working(state, &task) {
        return session::select(state, services, spawner, &id);
    }
    let now = Timestamp::now();
    let session = task_session(&task, now);
    let id = session.id.clone();
    state.session.open(session.clone(), now);
    crate::workspace::follow(state, spawner);
    follow(state, spawner);
    agent::start(state, spawner, id, session::FIRST_SIZE);
    let service = services.session.clone();
    session::record(spawner, session::NO_PENDING, async move {
        service.create_task(&session, &task, now).await
    });
    session::list(services, spawner);
}

/// The task of the selected session, read once with its body.
pub fn follow(state: &mut AppState, spawner: &dyn Spawner) {
    let Some(open) = state.session.selected() else {
        return;
    };
    let SessionKind::Task { external_id } = &open.session.kind else {
        return;
    };
    let external_id = external_id.clone();
    let known = state
        .task
        .by_external(&external_id)
        .is_some_and(|task| state.task.body(&task.short_id).is_some());
    if known {
        return;
    }
    match TaskKey::parse(&external_id) {
        Ok(key) => sync(state, spawner, key),
        Err(e) => state.errors.push(e),
    }
}

/// The open session working this task, if one already is.
fn working(state: &AppState, task: &Task) -> Option<SessionId> {
    state
        .session
        .open
        .iter()
        .find(|open| match &open.session.kind {
            SessionKind::Task { external_id } => *external_id == task.external_id,
            _ => false,
        })
        .map(|open| open.session.id.clone())
}

/// Reads every source in a job; the continuation puts the list in the slice.
pub fn load(state: &mut AppState, spawner: &dyn Spawner) {
    if state.task.reading {
        return;
    }
    let sources = sources(state.config.config.as_ref());
    if sources.is_empty() {
        return;
    }
    state.task.reading = true;
    let job = state.begin("reading the tasks");
    spawner.spawn(Box::pin(async move {
        let read = list(&sources).await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            state.end(job);
            state.task.reading = false;
            match read {
                Ok(tasks) => state.task.loaded(tasks),
                Err(e) => state.errors.push(e),
            }
        }) as Continuation
    }));
}

/// One task read again, with its body, for the views that show it.
fn sync(state: &mut AppState, spawner: &dyn Spawner, key: TaskKey) {
    let id = key.external_id();
    if !state.task.syncing.insert(id.clone()) {
        return;
    }
    let sources = sources(state.config.config.as_ref());
    let job = state.begin(format!("reading {id}"));
    spawner.spawn(Box::pin(async move {
        let read = fetch(&sources, &key).await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            state.end(job);
            state.task.syncing.remove(&id);
            match read {
                Ok(read) => state.task.synced(read),
                Err(e) => state.errors.push(e),
            }
        }) as Continuation
    }));
}
