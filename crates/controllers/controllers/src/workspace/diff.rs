//! Reading the worktree: what changed, what is on screen, what a write to disk means.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

use groove_types::WorktreeId;
use groove_workspace_service::{changes, painted, summary};

use super::editor::{Head, reopen};
use super::{directory, selected, stale, worktree_dir};
use crate::spawn::coalesced;
use crate::{AppState, Continuation, Deliver, Services, Spawner};

/// The rows on screen: their files take their colours, the others give theirs up.
pub(super) fn show(state: &mut AppState, spawner: &dyn Spawner, rows: std::ops::Range<usize>) {
    if state.workspace.showing == rows {
        return;
    }
    state.workspace.showing = rows.clone();
    let wanted = state.workspace.over(rows);
    state
        .workspace
        .coloured
        .retain(|path, _| wanted.contains(path));
    let missing: Vec<String> = wanted
        .into_iter()
        .filter(|path| !state.workspace.coloured.contains_key(path))
        .collect();
    let Some(dir) = worktree_dir(state) else {
        return;
    };
    if missing.is_empty() {
        return;
    }
    spawner.spawn(Box::pin(async move {
        let read = painted(&dir, missing).await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            state.workspace.coloured.extend(read);
            state.workspace.moved();
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
    let Some(id) = state.session.selected.clone() else {
        return;
    };
    let Some(worktree) = state
        .session
        .selected()
        .and_then(|open| open.selected_worktree())
        .map(|worktree| worktree.id.clone())
    else {
        return;
    };
    let read = !state
        .session
        .selected()
        .is_some_and(|open| open.is_read(&worktree, &path));
    if let Some(open) = state.session.get_mut(&id) {
        open.mark(&worktree, &path, read);
    }
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
    if state.workspace.moved_git(paths) {
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
    let Some(worktree) = state
        .session
        .selected()
        .and_then(|open| open.selected_worktree())
    else {
        return;
    };
    let (id, dir) = (worktree.id.clone(), PathBuf::from(&worktree.path));
    let job = state.begin("changed files");
    spawner.spawn(Box::pin(async move {
        let files = summary(&dir).await;
        let read = match &files {
            Ok(files) => changes(&dir, files).await,
            Err(_) => Default::default(),
        };
        let gone = !dir.exists();
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            state.end(job);
            match files {
                Ok(files) => state.workspace.loaded(id, files, read),
                Err(_) if gone => {}
                Err(e) => state.failed(e),
            }
        }) as Continuation
    }));
}

/// The selected worktree read and watched, or nothing when none is selected.
pub fn follow(state: &mut AppState, spawner: &dyn Spawner) {
    let Some(worktree) = selected(state) else {
        return state.workspace.clear();
    };
    if stale(state) {
        state.workspace.opened = None;
        load(state, spawner);
    }
    if state.workspace.watching.as_ref() != Some(&worktree) {
        watch(state, spawner, worktree);
    }
}

pub(super) fn watch(state: &mut AppState, spawner: &dyn Spawner, worktree: WorktreeId) {
    let Some(dir) = directory(state, &worktree) else {
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
