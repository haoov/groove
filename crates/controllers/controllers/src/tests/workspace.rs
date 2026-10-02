//! The workspace controller, one file per feature, on the helpers they share.

mod blame;
mod commits;
mod diff;
mod editor;
mod gaps;
mod git;
mod paths;
mod search;

use crate::tests::fixture::{pooled_clone, services, sh, state, until, worktree};
use crate::{Command as Cmd, Services, SyncSpawner, dispatch, session, workspace};
use groove_types::{Caret, Edit, FileStatus, Motion};

/// The fixture's worktree with `a.txt` open, and where it sits.
pub(super) fn editing(
    state: &mut crate::AppState,
    services: &Services,
    spawner: &SyncSpawner,
) -> std::path::PathBuf {
    let dir = worktree(state, services, spawner);
    let file = std::path::Path::new(&dir).join("a.txt");
    std::fs::write(&file, "one\ntwo\n").unwrap();
    until(spawner, services, state, |s| !s.workspace.files.is_empty());
    dispatch(
        Cmd::Workspace(workspace::Command::OpenFile {
            at: None,
            path: "a.txt".into(),
        }),
        state,
        services,
        spawner,
    );
    until(spawner, services, state, |s| s.workspace.active().is_some());
    file
}

pub(super) fn edit(
    state: &mut crate::AppState,
    services: &Services,
    spawner: &SyncSpawner,
    edits: &[Edit],
) {
    for one in edits {
        dispatch(
            Cmd::Workspace(workspace::Command::Edit(one.clone())),
            state,
            services,
            spawner,
        );
    }
}

pub(super) fn buffer(state: &crate::AppState) -> String {
    state
        .workspace
        .active()
        .map(|open| open.new.text())
        .expect("a file is open")
}

/// What the summary says is changed, and whether the index holds it.
pub(super) fn changed(state: &crate::AppState) -> Vec<(String, Option<bool>)> {
    let mut rows: Vec<(String, Option<bool>)> = state
        .workspace
        .files
        .iter()
        .map(|file| (file.path.clone(), file.staged))
        .collect();
    rows.sort();
    rows
}

pub(super) fn act(
    state: &mut crate::AppState,
    services: &Services,
    spawner: &SyncSpawner,
    command: workspace::Command,
) {
    dispatch(Cmd::Workspace(command), state, services, spawner);
}
