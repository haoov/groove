//! What a session's own actions leave on its log.

use groove_types::TimelineKind;

use super::fixture::{pooled_clone, services, state, until, worktree};
use crate::{Command as Cmd, Services, SyncSpawner, dispatch, workspace};

/// Every line the selected session's log holds, newest first.
fn log(
    state: &crate::AppState,
    services: &Services,
    spawner: &SyncSpawner,
) -> Vec<(TimelineKind, String)> {
    let session = state.session.selected.clone().expect("a session");
    let timeline = services.timeline.clone();
    spawner
        .block_on(async move { timeline.list(&session, 50).await })
        .expect("the log")
        .into_iter()
        .map(|one| (one.kind, one.subject))
        .collect()
}

fn send(
    state: &mut crate::AppState,
    services: &Services,
    spawner: &SyncSpawner,
    command: workspace::Command,
) {
    dispatch(Cmd::Workspace(command), state, services, spawner);
    until(spawner, services, state, |s| s.pending.is_empty());
}

#[test]
fn a_commit_leaves_its_subject_on_the_log() {
    let home = tempfile::tempdir().unwrap();
    pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    let dir = worktree(&mut state, &services, &spawner);
    std::fs::write(std::path::Path::new(&dir).join("a.txt"), "two\n").unwrap();
    send(&mut state, &services, &spawner, workspace::Command::Load);
    send(
        &mut state,
        &services,
        &spawner,
        workspace::Command::Stage {
            path: "a.txt".into(),
        },
    );
    state
        .workspace
        .message
        .edit(&groove_types::Edit::Insert("fix: one\n\nwhy".into()));

    send(&mut state, &services, &spawner, workspace::Command::Commit);
    assert!(state.errors.is_empty(), "{:?}", state.errors);
    assert_eq!(
        log(&state, &services, &spawner).first(),
        Some(&(TimelineKind::Commit, "fix: one".to_string())),
        "the subject alone, not the body"
    );
}

#[test]
fn a_worktree_made_says_so_on_the_log() {
    let home = tempfile::tempdir().unwrap();
    pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    worktree(&mut state, &services, &spawner);
    let read = log(&state, &services, &spawner);
    assert_eq!(read.len(), 1, "{read:?}");
    assert_eq!(read[0].0, TimelineKind::WorktreeAdded);
    assert!(read[0].1.starts_with("explorer/"), "the branch it made");
}

#[test]
fn a_tool_the_agent_ran_on_a_file_leaves_nothing() {
    let home = tempfile::tempdir().unwrap();
    pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    let dir = worktree(&mut state, &services, &spawner);
    let before = log(&state, &services, &spawner).len();
    std::fs::write(std::path::Path::new(&dir).join("a.txt"), "two\n").unwrap();
    send(&mut state, &services, &spawner, workspace::Command::Load);
    send(
        &mut state,
        &services,
        &spawner,
        workspace::Command::OpenFile {
            path: "a.txt".into(),
            at: None,
        },
    );
    send(
        &mut state,
        &services,
        &spawner,
        workspace::Command::Edit(groove_types::Edit::Insert("x".into())),
    );
    send(
        &mut state,
        &services,
        &spawner,
        workspace::Command::SaveFile,
    );
    assert_eq!(
        log(&state, &services, &spawner).len(),
        before,
        "reading and writing a file is status, not history"
    );
}
