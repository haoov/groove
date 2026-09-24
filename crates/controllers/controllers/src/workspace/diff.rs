//! Reading the worktree: what changed, what is on screen, what a write to disk means.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

use groove_types::{DiffMode, RowKind, WorktreeId};
use groove_workspace_service::{At, HEAD, base_rev, changes, painted, summary, summary_against};

use super::editor::{Head, reopen};
use super::{directory, selected, stale, worktree_dir};
use crate::spawn::coalesced;
use crate::{AppState, Continuation, Deliver, Services, Spawner};

/// How far past the rows on screen a file is read at.
const AHEAD: usize = 200;

/// The rows on screen and the ones around them: their files take their colours.
pub(super) fn show(state: &mut AppState, spawner: &dyn Spawner, rows: std::ops::Range<usize>) {
    if state.workspace.showing == rows {
        return;
    }
    state.workspace.showing = rows.clone();
    let near = rows.start.saturating_sub(AHEAD)..rows.end + AHEAD;
    let wanted = state.workspace.over(near);
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
    let read_for = read_for_of(state);
    let (mode, base) = (read_for.1, read_for.2.clone());
    spawner.spawn(Box::pin(async move {
        let rev = against(&dir, mode, base.as_deref()).await;
        let read = painted(&dir, missing, &rev).await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            if read_for_of(state) == read_for {
                state.workspace.coloured.extend(read);
            }
        }) as Continuation
    }));
}

/// The diff a read is for: its worktree, its mode, the branch it is read against.
type ReadFor = (Option<WorktreeId>, DiffMode, Option<String>);

fn read_for_of(state: &AppState) -> ReadFor {
    let worktree = state.workspace.worktree.clone();
    (worktree, state.workspace.mode, super::selected_base(state))
}

/// How many lines one click of a gap gives up.
const STEP: u32 = 20;

/// The gap on this row gives up its lines, from one end or whole.
pub(super) fn open_gap(state: &mut AppState, row: usize, way: super::Way) {
    let Some((path, span)) = gap_at(state, row) else {
        return;
    };
    let wanted = match way {
        super::Way::All => span,
        super::Way::Down => span.start..(span.start + STEP).min(span.end),
        super::Way::Up => span.end.saturating_sub(STEP).max(span.start)..span.end,
    };
    state.workspace.open_gap(&path, wanted);
}

/// The file a gap row belongs to, and the old-side lines it hides.
fn gap_at(state: &AppState, row: usize) -> Option<(String, std::ops::Range<u32>)> {
    let At::Row(file, at) = state.workspace.changes.at(row)? else {
        return None;
    };
    let RowKind::Gap(lines) = file.rows[at].kind else {
        return None;
    };
    let start = file.rows[..at]
        .iter()
        .rev()
        .find_map(|row| row.old)
        .map(|old| old + 1)
        .unwrap_or_default();
    Some((file.path.clone(), start..start + lines))
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
    let base = worktree.base_ref.clone();
    let mode = state.workspace.mode;
    let job = state.begin("changed files");
    spawner.spawn(Box::pin(async move {
        let rev = against(&dir, mode, base.as_deref()).await;
        let files = files_in(&dir, mode, &rev).await;
        let read = match &files {
            Ok(files) => changes(&dir, files, &rev).await,
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
    let Some(worktree) = selected(state) else {
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
