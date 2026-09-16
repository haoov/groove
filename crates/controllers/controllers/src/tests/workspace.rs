use groove_types::FileStatus;

use crate::tests::fixture::{pooled_clone, services, state, until, worktree};
use crate::{Command as Cmd, SyncSpawner, dispatch, session, workspace};

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

#[test]
fn a_file_changing_on_disk_reads_the_worktree_again() {
    let home = tempfile::tempdir().unwrap();
    pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    let dir = worktree(&mut state, &services, &spawner);
    assert!(
        state.workspace.watching.is_some(),
        "the selected worktree is watched"
    );

    std::fs::write(std::path::Path::new(&dir).join("a.txt"), "changed\n").unwrap();
    until(&spawner, &services, &mut state, |s| {
        !s.workspace.files.is_empty()
    });
    assert_eq!(state.workspace.files[0].path, "a.txt");

    let second = state.session.selected.clone().expect("a session");
    dispatch(
        Cmd::Session(session::Command::Close { session: second }),
        &mut state,
        &services,
        &spawner,
    );
    assert!(
        state.workspace.watching.is_none(),
        "nothing selected, nothing watched"
    );
}

#[test]
fn the_open_file_follows_a_change_on_disk() {
    let home = tempfile::tempdir().unwrap();
    pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    let dir = worktree(&mut state, &services, &spawner);
    let file = std::path::Path::new(&dir).join("a.txt");

    std::fs::write(&file, "two\n").unwrap();
    until(&spawner, &services, &mut state, |s| {
        !s.workspace.files.is_empty()
    });
    dispatch(
        Cmd::Workspace(workspace::Command::OpenFile {
            path: "a.txt".into(),
        }),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| {
        s.workspace.opened.is_some()
    });
    let rows = state.workspace.opened.as_ref().map(|open| open.rows.len());
    assert_eq!(rows, Some(2), "one line out, one line in");

    std::fs::write(&file, "two\nthree\nfour\n").unwrap();
    until(&spawner, &services, &mut state, |s| {
        s.workspace
            .opened
            .as_ref()
            .is_some_and(|open| open.new.lines() == 3)
    });
    let open = state.workspace.opened.as_ref().expect("still open");
    assert_eq!(open.path, "a.txt", "the same file, read again");
    assert_eq!(open.rows.len(), 4, "one out, three in");
}
