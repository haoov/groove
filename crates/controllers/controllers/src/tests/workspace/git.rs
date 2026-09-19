//! The index, the commit, the push and the pull, as the controller drives them.

use super::*;

#[test]
fn a_file_is_staged_then_taken_back_out() {
    let home = tempfile::tempdir().unwrap();
    pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    let dir = worktree(&mut state, &services, &spawner);
    std::fs::write(std::path::Path::new(&dir).join("a.txt"), "changed\n").unwrap();
    until(&spawner, &services, &mut state, |s| {
        !s.workspace.files.is_empty()
    });
    assert_eq!(changed(&state), [("a.txt".to_string(), Some(false))]);

    act(
        &mut state,
        &services,
        &spawner,
        workspace::Command::Stage {
            path: "a.txt".into(),
        },
    );
    until(&spawner, &services, &mut state, |s| {
        s.workspace.files.iter().any(|f| f.staged == Some(true))
    });
    assert_eq!(changed(&state), [("a.txt".to_string(), Some(true))]);

    act(
        &mut state,
        &services,
        &spawner,
        workspace::Command::Unstage {
            path: "a.txt".into(),
        },
    );
    until(&spawner, &services, &mut state, |s| {
        s.workspace.files.iter().any(|f| f.staged == Some(false))
    });
    assert_eq!(changed(&state), [("a.txt".to_string(), Some(false))]);
    assert!(state.errors.is_empty(), "{:?}", state.errors);
}

#[test]
fn discarding_a_file_puts_it_back_and_takes_it_off_the_list() {
    let home = tempfile::tempdir().unwrap();
    pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    let dir = worktree(&mut state, &services, &spawner);
    let file = std::path::Path::new(&dir).join("a.txt");
    let before = std::fs::read_to_string(&file).unwrap();
    std::fs::write(&file, "changed\n").unwrap();
    until(&spawner, &services, &mut state, |s| {
        !s.workspace.files.is_empty()
    });

    act(
        &mut state,
        &services,
        &spawner,
        workspace::Command::Discard {
            path: "a.txt".into(),
        },
    );
    until(&spawner, &services, &mut state, |s| {
        s.workspace.files.is_empty()
    });
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        before,
        "the file is what HEAD has"
    );
}

#[test]
fn a_commit_takes_the_index_and_empties_the_message() {
    let home = tempfile::tempdir().unwrap();
    pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    let dir = worktree(&mut state, &services, &spawner);
    std::fs::write(std::path::Path::new(&dir).join("a.txt"), "changed\n").unwrap();
    until(&spawner, &services, &mut state, |s| {
        !s.workspace.files.is_empty()
    });
    act(
        &mut state,
        &services,
        &spawner,
        workspace::Command::Stage {
            path: "a.txt".into(),
        },
    );
    until(&spawner, &services, &mut state, |s| {
        s.workspace.files.iter().any(|f| f.staged == Some(true))
    });

    state
        .workspace
        .message
        .edit(&Edit::Insert("fix(a): change it".into()));
    act(&mut state, &services, &spawner, workspace::Command::Commit);
    until(&spawner, &services, &mut state, |s| {
        s.workspace.files.is_empty()
    });
    assert_eq!(
        state.workspace.message.text(),
        "",
        "the message is spent with the commit"
    );
    assert!(state.errors.is_empty(), "{:?}", state.errors);
}

#[test]
fn a_commit_with_no_message_is_not_made() {
    let home = tempfile::tempdir().unwrap();
    pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    worktree(&mut state, &services, &spawner);
    state.workspace.message.edit(&Edit::Insert("   ".into()));
    act(&mut state, &services, &spawner, workspace::Command::Commit);
    spawner.drain(&mut state, &services);
    assert!(state.errors.is_empty(), "nothing was tried");
    assert_eq!(state.workspace.message.text(), "   ", "and nothing spent");
}

#[test]
fn a_stage_from_outside_the_window_reaches_the_list() {
    let home = tempfile::tempdir().unwrap();
    pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    let dir = worktree(&mut state, &services, &spawner);
    std::fs::write(std::path::Path::new(&dir).join("a.txt"), "changed\n").unwrap();
    until(&spawner, &services, &mut state, |s| {
        !s.workspace.files.is_empty()
    });
    assert_eq!(changed(&state), [("a.txt".to_string(), Some(false))]);

    sh(std::path::Path::new(&dir), &["add", "a.txt"]);
    until(&spawner, &services, &mut state, |s| {
        s.workspace.files.iter().any(|f| f.staged == Some(true))
    });
    assert_eq!(
        changed(&state),
        [("a.txt".to_string(), Some(true))],
        "git wrote its index and the window noticed"
    );
}

#[test]
fn a_commit_from_outside_the_window_reaches_the_diff() {
    let home = tempfile::tempdir().unwrap();
    pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    let dir = worktree(&mut state, &services, &spawner);
    let at = std::path::Path::new(&dir);
    std::fs::write(at.join("a.txt"), "changed\n").unwrap();
    until(&spawner, &services, &mut state, |s| {
        !s.workspace.files.is_empty()
    });

    sh(at, &["add", "a.txt"]);
    sh(at, &["commit", "-m", "fix(a): change it"]);
    until(&spawner, &services, &mut state, |s| {
        s.workspace.files.is_empty()
    });
    assert!(
        state.workspace.files.is_empty(),
        "nothing is changed any more, since HEAD has it"
    );
}

#[test]
fn a_commit_is_pushed_and_the_branch_stops_being_ahead() {
    let home = tempfile::tempdir().unwrap();
    pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    let dir = worktree(&mut state, &services, &spawner);
    let at = std::path::Path::new(&dir);
    std::fs::write(at.join("a.txt"), "changed\n").unwrap();
    until(&spawner, &services, &mut state, |s| {
        !s.workspace.files.is_empty()
    });
    sh(at, &["add", "a.txt"]);
    sh(at, &["commit", "-m", "fix(a): change it"]);
    until(&spawner, &services, &mut state, |s| {
        s.workspace.files.is_empty()
    });

    act(&mut state, &services, &spawner, workspace::Command::Push);
    until(&spawner, &services, &mut state, |s| s.pending.is_empty());
    assert!(state.errors.is_empty(), "{:?}", state.errors);
    let pushed = sh(at, &["rev-parse", "HEAD"]);
    let upstream = sh(at, &["rev-parse", "@{upstream}"]);
    assert_eq!(pushed, upstream, "origin has what the branch has");
}

#[test]
fn discarding_everything_leaves_the_worktree_as_head_has_it() {
    let home = tempfile::tempdir().unwrap();
    pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    let dir = worktree(&mut state, &services, &spawner);
    let at = std::path::Path::new(&dir);
    let before = std::fs::read_to_string(at.join("a.txt")).unwrap();
    std::fs::write(at.join("a.txt"), "changed\n").unwrap();
    std::fs::write(at.join("new.txt"), "fresh\n").unwrap();
    until(&spawner, &services, &mut state, |s| {
        s.workspace.files.len() == 2
    });

    act(
        &mut state,
        &services,
        &spawner,
        workspace::Command::DiscardAll,
    );
    until(&spawner, &services, &mut state, |s| {
        s.workspace.files.is_empty()
    });
    assert_eq!(std::fs::read_to_string(at.join("a.txt")).unwrap(), before);
    assert!(!at.join("new.txt").exists(), "and the untracked one goes");
}

#[test]
fn pulling_a_branch_that_never_moved_says_nothing_went_wrong() {
    let home = tempfile::tempdir().unwrap();
    pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    worktree(&mut state, &services, &spawner);
    act(&mut state, &services, &spawner, workspace::Command::Pull);
    until(&spawner, &services, &mut state, |s| s.pending.is_empty());
    assert!(state.errors.is_empty(), "{:?}", state.errors);
}
