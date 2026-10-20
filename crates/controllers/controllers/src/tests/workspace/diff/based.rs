//! What the base mode holds against the working tree.

use super::*;

#[test]
fn the_base_mode_holds_what_the_branch_committed_and_working_does_not() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    let dir = worktree(&mut state, &services, &spawner);
    let at = std::path::Path::new(&dir);
    based(&mut state, "main");

    std::fs::write(at.join("a.txt"), "one\ntwo\n").unwrap();
    sh(at, &["add", "-A"]);
    sh(at, &["commit", "-m", "fix: one"]);
    dispatch(
        Cmd::Workspace(workspace::Command::Load),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| s.pending.is_empty());
    assert!(
        state.workspace.files.is_empty(),
        "working holds only what is uncommitted: {:?}",
        state.workspace.files
    );

    dispatch(
        Cmd::Workspace(workspace::Command::SetMode {
            mode: groove_types::DiffMode::Base,
        }),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| {
        s.pending.is_empty() && !s.workspace.files.is_empty()
    });
    let files = &state.workspace.files;
    assert_eq!(files.len(), 1, "{files:?}");
    assert_eq!(files[0].path, "a.txt");
    assert_eq!(files[0].status, FileStatus::Modified);
    assert!(state.errors.is_empty(), "{:?}", state.errors);
}

#[test]
fn the_base_mode_holds_what_is_uncommitted_as_well() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    let dir = worktree(&mut state, &services, &spawner);
    let at = std::path::Path::new(&dir);
    based(&mut state, "main");

    std::fs::write(at.join("a.txt"), "one\ntwo\n").unwrap();
    sh(at, &["add", "-A"]);
    sh(at, &["commit", "-m", "fix: one"]);
    std::fs::write(at.join("b.txt"), "fresh\n").unwrap();

    dispatch(
        Cmd::Workspace(workspace::Command::SetMode {
            mode: groove_types::DiffMode::Base,
        }),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| {
        s.pending.is_empty() && s.workspace.files.len() > 1
    });
    let paths: Vec<&str> = state
        .workspace
        .files
        .iter()
        .map(|file| file.path.as_str())
        .collect();
    assert_eq!(
        paths,
        ["a.txt", "b.txt"],
        "the commit and the untracked file"
    );
}
