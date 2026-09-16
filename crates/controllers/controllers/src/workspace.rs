//! The `workspace` controller: one function per user action on the `workspace` service.

use std::path::PathBuf;

use groove_types::WorktreeId;
use groove_workspace_service::summary;

use crate::{AppState, Continuation, Services, Spawner};

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// `workspace.load`: the selected worktree's changed files.
    Load,
}

impl Command {
    pub fn id(&self) -> &'static str {
        match self {
            Command::Load => "workspace.load",
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
        Command::Load => load(state, spawner),
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
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            state.end(job);
            match files {
                Ok(files) => state.workspace.loaded(id, files),
                Err(e) => state.errors.push(e),
            }
        }) as Continuation
    }));
}

/// What every change of selection asks for: the new worktree's files, or none.
pub fn load_if_stale(state: &mut AppState, spawner: &dyn Spawner) {
    if selected(state).is_none() {
        return state.workspace.clear();
    }
    if stale(state) {
        load(state, spawner);
    }
}

/// Nothing is loaded for the selected worktree yet.
pub fn stale(state: &AppState) -> bool {
    match selected(state) {
        Some(worktree) => !state.workspace.holds(&worktree),
        None => false,
    }
}

/// The worktree every tab follows.
fn selected(state: &AppState) -> Option<WorktreeId> {
    let open = state.session.selected()?;
    Some(open.selected_worktree()?.id.clone())
}

/// The id the summary belongs to, for a test to read.
pub fn loaded_for(state: &AppState) -> Option<&WorktreeId> {
    state.workspace.worktree.as_ref()
}
