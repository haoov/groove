//! Reading the worktree: what changed, what is on screen, what a write to disk means.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

use groove_types::{DiffMode, WorktreeId};
use groove_workspace_service::{HEAD, base_rev, changes, painted, summary, summary_against};

use super::editor::{Head, reopen};
use super::stale;
use crate::spawn::coalesced;
use crate::{AppState, Continuation, Deliver, Services, Spawner};

/// The rows on screen: the files near them that are not read yet take their colours.
pub(super) fn show(state: &mut AppState, spawner: &dyn Spawner, rows: std::ops::Range<usize>) {
    let missing = state.workspace.show(rows);
    let Some(asked) = super::Asked::now(state).filter(|_| !missing.is_empty()) else {
        return;
    };
    spawner.spawn(Box::pin(async move {
        let rev = against(&asked.dir, asked.mode, asked.base.as_deref()).await;
        let read = painted(&asked.dir, missing, &rev).await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            if asked.holds(state) {
                state.workspace.coloured.extend(read);
            }
        }) as Continuation
    }));
}

/// One file read or unread: the session says so at once, and the disk follows.
pub(super) fn mark_read(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    path: String,
) {
    let Some(one) = state.session.selected_worktree() else {
        return;
    };
    let (id, worktree) = (one.session.clone(), one.id.clone());
    let Some(open) = state.session.get_mut(&id) else {
        return;
    };
    let read = open.toggle_read(&worktree, &path);
    let service = services.session.clone();
    spawner.spawn(Box::pin(async move {
        let done = service.set_read(&id, &worktree, &path, read).await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            if let Err(e) = done {
                state.failed(e);
            }
        }) as Continuation
    }));
}

/// The worktree and the file it is showing, read at the same time: the file's rows do
/// not wait on the summary.
pub(super) fn reread(state: &mut AppState, spawner: &dyn Spawner) {
    load(state, spawner);
    reopen(state, spawner, Head::Read);
}

/// The same, for what a write under the worktree touched.
pub(super) fn refresh(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    paths: &[PathBuf],
) {
    if groove_workspace_service::moved_git(paths) {
        crate::session::refresh_status(state, services, spawner);
        return reread(state, spawner);
    }
    load(state, spawner);
    if state.workspace.shows(paths) {
        reopen(state, spawner, Head::Keep);
    }
}

/// Reads the summary in a job; the continuation stores it against its worktree.
pub fn load(state: &mut AppState, spawner: &dyn Spawner) {
    let Some(asked) = super::Asked::now(state) else {
        return;
    };
    let job = state.begin("changed files");
    spawner.spawn(Box::pin(async move {
        let (dir, mode) = (&asked.dir, asked.mode);
        let rev = against(dir, mode, asked.base.as_deref()).await;
        let files = files_in(dir, mode, &rev).await;
        let read = match &files {
            Ok(files) => changes(dir, files, &rev).await,
            Err(_) => Default::default(),
        };
        let gone = !dir.exists();
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            state.end(job);
            if !asked.holds(state) {
                return;
            }
            match files {
                Ok(files) => state.workspace.loaded(asked.worktree, files, read),
                Err(_) if gone => {}
                Err(e) => state.failed(e),
            }
        }) as Continuation
    }));
}

/// What the change is read against; the rows and the open file follow it.
pub(super) fn set_mode(state: &mut AppState, spawner: &dyn Spawner, mode: DiffMode) {
    if state.workspace.mode == mode {
        return;
    }
    state.workspace.mode = mode;
    reread(state, spawner);
}

/// What changed in a worktree, read the way the mode asks for it.
pub(crate) async fn files_in(
    dir: &std::path::Path,
    mode: DiffMode,
    rev: &str,
) -> groove_types::Result<Vec<groove_types::FileDiff>> {
    match mode {
        DiffMode::Working => summary(dir).await,
        _ => summary_against(dir, rev).await,
    }
}

/// The rev the change is read against, for the mode in hand.
pub(crate) async fn against(dir: &std::path::Path, mode: DiffMode, base: Option<&str>) -> String {
    match mode {
        DiffMode::Working => HEAD.to_string(),
        _ => base_rev(dir, base).await,
    }
}

/// The selected worktree read and watched, or nothing when none is selected.
pub fn follow(state: &mut AppState, spawner: &dyn Spawner) {
    let Some(worktree) = state.session.selected_worktree().map(|one| one.id.clone()) else {
        return state.workspace.clear();
    };
    if stale(state) {
        state.workspace.mode = opens_in(state);
        state.workspace.opened = None;
        load(state, spawner);
    }
    if state.workspace.watching.as_ref() != Some(&worktree) {
        watch(state, spawner, worktree);
    }
}

/// A review opens on the whole change; every other session on what is not committed.
fn opens_in(state: &AppState) -> DiffMode {
    match state.session.selected().map(|open| &open.session.kind) {
        Some(groove_types::SessionKind::Review { .. }) => DiffMode::Base,
        _ => DiffMode::Working,
    }
}

pub(super) fn watch(state: &mut AppState, spawner: &dyn Spawner, worktree: WorktreeId) {
    let Some(dir) = state.session.find(&worktree).map(|(_, _, one)| one.dir()) else {
        return;
    };
    spawner.spawn(Box::pin(async move {
        let git = groove_workspace_service::git_dir(&dir).await;
        Box::new(
            move |state: &mut AppState, _: &Services, spawner: &dyn Spawner| {
                if !dir.exists() {
                    return;
                }
                let on_change = reload(spawner.sink(), worktree.clone());
                let watched = groove_workspace_service::watch(
                    &mut state.workspace,
                    worktree,
                    &dir,
                    git,
                    on_change,
                );
                if let Err(e) = watched {
                    state.failed(e);
                }
            },
        ) as Continuation
    }));
}

/// One read in flight at most. What moves while it runs waits for the next one.
pub(super) fn reload(
    sink: Arc<dyn Deliver>,
    worktree: WorktreeId,
) -> impl Fn(Vec<PathBuf>) + Send + Sync + 'static {
    let changed: Arc<Mutex<Vec<PathBuf>>> = Arc::default();
    let queue = changed.clone();
    let once = coalesced(sink, move || {
        let (worktree, changed) = (worktree.clone(), changed.clone());
        Box::new(
            move |state: &mut AppState, services: &Services, spawner: &dyn Spawner| {
                let paths = std::mem::take(&mut *lock(&changed));
                if state.workspace.watching.as_ref() == Some(&worktree) {
                    refresh(state, services, spawner, &paths);
                }
            },
        ) as Continuation
    });
    move |paths: Vec<PathBuf>| {
        lock(&queue).extend(paths);
        once();
    }
}

fn lock(paths: &Mutex<Vec<PathBuf>>) -> std::sync::MutexGuard<'_, Vec<PathBuf>> {
    paths.lock().unwrap_or_else(PoisonError::into_inner)
}
