//! The branch's commits, one of them shown, and the writes it refuses.

use groove_types::{Caret, Edit, Motion};

use super::*;

/// The fixture with a worktree and its two commits read.
fn logged(
    home: &std::path::Path,
    spawner: &SyncSpawner,
) -> (crate::AppState, Services, std::path::PathBuf) {
    pooled_clone(home);
    let services = services(spawner, home);
    let mut state = state(home);
    let dir = worktree(&mut state, &services, spawner);
    send(
        &mut state,
        &services,
        spawner,
        workspace::Command::GetCommits,
    );
    (state, services, dir.into())
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
fn the_log_holds_the_commits_of_the_branch() {
    let home = tempfile::tempdir().unwrap();
    let spawner = SyncSpawner::new().unwrap();
    let (state, _, _) = logged(home.path(), &spawner);
    assert!(state.errors.is_empty(), "{:?}", state.errors);
    let said: Vec<&str> = state
        .workspace
        .log
        .iter()
        .map(|one| one.message.as_str())
        .collect();
    assert_eq!(said, ["first"], "the seed's own commit");
}

#[test]
fn a_commit_opened_shows_what_it_changed() {
    let home = tempfile::tempdir().unwrap();
    let spawner = SyncSpawner::new().unwrap();
    let (mut state, services, dir) = logged(home.path(), &spawner);
    std::fs::write(dir.join("b.txt"), "b\n").unwrap();
    sh(&dir, &["add", "."]);
    sh(&dir, &["commit", "-m", "second"]);
    send(
        &mut state,
        &services,
        &spawner,
        workspace::Command::GetCommits,
    );
    let sha = state.workspace.log[0].sha.clone();

    send(
        &mut state,
        &services,
        &spawner,
        workspace::Command::OpenCommit { sha: sha.clone() },
    );
    assert!(state.errors.is_empty(), "{:?}", state.errors);
    assert_eq!(
        state.workspace.commit.as_ref().map(|one| one.sha.clone()),
        Some(sha)
    );
    assert!(state.workspace.readonly(), "a commit is never written");
    let paths: Vec<&str> = state
        .workspace
        .files
        .iter()
        .map(|file| file.path.as_str())
        .collect();
    assert_eq!(paths, ["b.txt"], "the file the commit added");
}

#[test]
fn nothing_writes_while_a_commit_is_shown() {
    let home = tempfile::tempdir().unwrap();
    let spawner = SyncSpawner::new().unwrap();
    let (mut state, services, dir) = logged(home.path(), &spawner);
    let file = dir.join("a.txt");
    std::fs::write(&file, "one\ntwo\n").unwrap();
    send(&mut state, &services, &spawner, workspace::Command::Load);
    let sha = state.workspace.log[0].sha.clone();
    send(
        &mut state,
        &services,
        &spawner,
        workspace::Command::OpenCommit { sha },
    );

    for one in [
        workspace::Command::Edit(Edit::Insert("x".into())),
        workspace::Command::SaveFile,
        workspace::Command::Stage {
            path: "a.txt".into(),
        },
        workspace::Command::Discard {
            path: "a.txt".into(),
        },
        workspace::Command::DiscardAll,
        workspace::Command::Commit,
    ] {
        send(&mut state, &services, &spawner, one);
    }
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        "one\ntwo\n",
        "the worktree is untouched"
    );
    assert!(state.errors.is_empty(), "{:?}", state.errors);
}

#[test]
fn leaving_the_commit_brings_the_working_tree_back() {
    let home = tempfile::tempdir().unwrap();
    let spawner = SyncSpawner::new().unwrap();
    let (mut state, services, dir) = logged(home.path(), &spawner);
    std::fs::write(dir.join("a.txt"), "one\ntwo\n").unwrap();
    send(&mut state, &services, &spawner, workspace::Command::Load);
    let sha = state.workspace.log[0].sha.clone();
    send(
        &mut state,
        &services,
        &spawner,
        workspace::Command::OpenCommit { sha },
    );
    assert!(state.workspace.readonly());

    send(
        &mut state,
        &services,
        &spawner,
        workspace::Command::LeaveCommit,
    );
    assert!(!state.workspace.readonly(), "the working tree is writable");
    assert_eq!(changed(&state), [("a.txt".to_string(), Some(false))]);

    dispatch(
        Cmd::Workspace(workspace::Command::OpenFile {
            path: "a.txt".into(),
            at: None,
        }),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| {
        s.workspace.opened.is_some()
    });
    edit(
        &mut state,
        &services,
        &spawner,
        &[
            Edit::Move(Motion::To(Caret::new(0, 0))),
            Edit::Insert("x".into()),
        ],
    );
    assert_eq!(buffer(&state), "xone\ntwo\n", "and takes an edit again");
}

#[test]
fn a_file_of_a_commit_reads_as_that_commit_left_it() {
    let home = tempfile::tempdir().unwrap();
    let spawner = SyncSpawner::new().unwrap();
    let (mut state, services, dir) = logged(home.path(), &spawner);
    std::fs::write(dir.join("a.txt"), "second\n").unwrap();
    sh(&dir, &["commit", "-am", "second"]);
    std::fs::write(dir.join("a.txt"), "working\n").unwrap();
    send(&mut state, &services, &spawner, workspace::Command::GetCommits);
    let sha = state.workspace.log[0].sha.clone();

    send(
        &mut state,
        &services,
        &spawner,
        workspace::Command::OpenCommit { sha },
    );
    send(
        &mut state,
        &services,
        &spawner,
        workspace::Command::OpenFile {
            path: "a.txt".into(),
            at: None,
        },
    );
    assert_eq!(
        buffer(&state),
        "second\n",
        "the commit's own side, not what the disk holds now"
    );
    let old = state
        .workspace
        .opened
        .as_ref()
        .map(|open| open.old.text())
        .expect("the open file");
    assert_eq!(old, "one\n", "against what its parent had");
}
