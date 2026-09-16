//! The `workspace` controller: one function per user action on the `workspace` service.

use std::path::PathBuf;
use std::sync::Arc;

use groove_types::WorktreeId;
use groove_workspace_service::{opened, summary};

use crate::spawn::coalesced;
use crate::{AppState, Continuation, Deliver, Services, Spawner};

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// `workspace.load`: the selected worktree's changed files.
    Load,
    /// `workspace.open_file`: one file's two sides and the rows between them.
    OpenFile { path: String },
}

impl Command {
    pub fn id(&self) -> &'static str {
        match self {
            Command::Load => "workspace.load",
            Command::OpenFile { .. } => "workspace.open_file",
        }
    }
}

pub fn dispatch(
    command: Command,
    state: &mut AppState,
    _services: &Services,
    spawner: &dyn Spawner,
) {
    match command {
        Command::Load => reread(state, spawner),
        Command::OpenFile { path } => open_file(state, spawner, path),
    }
}

/// The worktree and the file it is showing, read at the same time: the file's rows do
/// not wait on the summary.
fn reread(state: &mut AppState, spawner: &dyn Spawner) {
    load(state, spawner);
    reopen(state, spawner);
}

/// Reads the open file again, now the worktree has moved under it.
fn reopen(state: &mut AppState, spawner: &dyn Spawner) {
    let Some(path) = state
        .workspace
        .opened
        .as_ref()
        .map(|open| open.path.clone())
    else {
        return;
    };
    open_file(state, spawner, path);
}

/// Reads both sides in a job; the continuation stores them for the tab to draw.
pub fn open_file(state: &mut AppState, spawner: &dyn Spawner, path: String) {
    let Some(dir) = worktree_dir(state) else {
        return;
    };
    let job = state.begin(format!("opening {path}"));
    spawner.spawn(Box::pin(async move {
        let file = opened(&dir, &path).await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            state.end(job);
            match file {
                Ok(file) => state.workspace.opened = Some(file),
                Err(e) => state.errors.push(e),
            }
        }) as Continuation
    }));
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
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            state.end(job);
            match files {
                Ok(files) => state.workspace.loaded(id, files),
                Err(e) => state.errors.push(e),
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
        load(state, spawner);
    }
    if state.workspace.watching.as_ref() != Some(&worktree) {
        watch(state, spawner, worktree);
    }
}

fn watch(state: &mut AppState, spawner: &dyn Spawner, worktree: WorktreeId) {
    let Some(dir) = directory(state, &worktree) else {
        return;
    };
    let on_change = reload(spawner.sink(), worktree.clone());
    let watched =
        groove_workspace_service::watch(&mut state.workspace, worktree, &dir, move |_paths| {
            on_change()
        });
    if let Err(e) = watched {
        state.errors.push(e);
    }
}

/// One read in flight at most.
fn reload(sink: Arc<dyn Deliver>, worktree: WorktreeId) -> impl Fn() + Send + Sync {
    coalesced(sink, move || {
        let worktree = worktree.clone();
        Box::new(
            move |state: &mut AppState, _: &Services, spawner: &dyn Spawner| {
                if state.workspace.watching.as_ref() == Some(&worktree) {
                    reread(state, spawner);
                }
            },
        ) as Continuation
    })
}

/// Where the selected worktree sits on disk.
fn worktree_dir(state: &AppState) -> Option<PathBuf> {
    let open = state.session.selected()?;
    Some(PathBuf::from(&open.selected_worktree()?.path))
}

fn directory(state: &AppState, worktree: &WorktreeId) -> Option<PathBuf> {
    let open = state.session.selected()?;
    let found = open.worktrees.iter().find(|w| &w.id == worktree)?;
    Some(PathBuf::from(&found.path))
}

pub fn stale(state: &AppState) -> bool {
    match selected(state) {
        Some(worktree) => !state.workspace.holds(&worktree),
        None => false,
    }
}

fn selected(state: &AppState) -> Option<WorktreeId> {
    let open = state.session.selected()?;
    Some(open.selected_worktree()?.id.clone())
}

pub fn loaded_for(state: &AppState) -> Option<&WorktreeId> {
    state.workspace.worktree.as_ref()
}
