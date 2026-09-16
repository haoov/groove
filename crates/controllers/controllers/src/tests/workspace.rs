use groove_types::{FileStatus, WorktreeSpec};

use crate::tests::fixture::{pooled_clone, services, state, until};
use crate::{Command as Cmd, SyncSpawner, dispatch, session, workspace};

/// An explorer with the fixture's repo and one worktree; returns the worktree's path.
fn worktree(
    state: &mut crate::AppState,
    services: &crate::Services,
    spawner: &SyncSpawner,
) -> String {
    dispatch(
        Cmd::Session(session::Command::OpenExplorer {
            title: Some("try mayo".into()),
        }),
        state,
        services,
        spawner,
    );
    let id = state.session.selected.clone().expect("a session");
    until(spawner, services, state, |s| s.agent.agent(&id).is_some());
    dispatch(
        Cmd::Session(session::Command::AddRepo {
            session: id.clone(),
            name: "mayo".into(),
            spec: WorktreeSpec::default(),
        }),
        state,
        services,
        spawner,
    );
    until(spawner, services, state, |s| {
        s.workspace.worktree.is_some() || !s.errors.is_empty()
    });
    assert!(state.errors.is_empty(), "{:?}", state.errors);
    state
        .session
        .get(&id)
        .and_then(|o| o.selected_worktree())
        .map(|w| w.path.clone())
        .expect("a worktree")
}

#[test]
fn load_lists_what_changed_in_the_selected_worktree() {
    let home = tempfile::tempdir().unwrap();
    pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    let dir = worktree(&mut state, &services, &spawner);
    assert!(
        workspace::loaded_for(&state).is_some(),
        "adding the repo loaded its worktree"
    );
    assert!(
        state.workspace.files.is_empty(),
        "a fresh worktree is clean"
    );
    assert!(!workspace::stale(&state));

    std::fs::write(std::path::Path::new(&dir).join("a.txt"), "one\ntwo\n").unwrap();
    std::fs::write(std::path::Path::new(&dir).join("new.txt"), "fresh\n").unwrap();
    dispatch(
        Cmd::Workspace(workspace::Command::Load),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| {
        !s.workspace.files.is_empty()
    });
    let files: Vec<(&str, FileStatus)> = state
        .workspace
        .files
        .iter()
        .map(|f| (f.path.as_str(), f.status))
        .collect();
    assert_eq!(
        files,
        [
            ("a.txt", FileStatus::Modified),
            ("new.txt", FileStatus::Untracked)
        ]
    );
}

#[test]
fn nothing_is_loaded_when_no_worktree_is_selected() {
    let home = tempfile::tempdir().unwrap();
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    dispatch(
        Cmd::Workspace(workspace::Command::Load),
        &mut state,
        &services,
        &spawner,
    );
    spawner.drain(&mut state, &services);
    assert!(workspace::loaded_for(&state).is_none());
    assert!(!workspace::stale(&state), "nothing to load is not stale");
}

#[test]
fn switching_to_a_session_with_no_worktree_forgets_the_last_one() {
    let home = tempfile::tempdir().unwrap();
    pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    let dir = worktree(&mut state, &services, &spawner);
    let first = state.session.selected.clone().expect("a session");
    std::fs::write(std::path::Path::new(&dir).join("a.txt"), "changed\n").unwrap();
    dispatch(
        Cmd::Workspace(workspace::Command::Load),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| {
        !s.workspace.files.is_empty()
    });

    dispatch(
        Cmd::Session(session::Command::OpenExplorer { title: None }),
        &mut state,
        &services,
        &spawner,
    );
    let second = state.session.selected.clone().expect("a second session");
    assert_ne!(first, second);
    dispatch(
        Cmd::Session(session::Command::Select {
            session: second.clone(),
        }),
        &mut state,
        &services,
        &spawner,
    );
    assert!(
        workspace::loaded_for(&state).is_none(),
        "a session with no worktree has no changed files"
    );
    assert!(state.workspace.files.is_empty());

    dispatch(
        Cmd::Session(session::Command::Select { session: first }),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| {
        !s.workspace.files.is_empty()
    });
    assert_eq!(state.workspace.files[0].path, "a.txt", "and back again");
}
