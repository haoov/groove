//! The worktree's own files listed, from the command to what lands in the state.

use super::*;

#[test]
fn the_worktrees_own_files_are_listed_and_one_that_never_changed_opens() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    worktree(&mut state, &services, &spawner);

    dispatch(
        Cmd::Workspace(workspace::Command::ListPaths),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| {
        !s.workspace.paths().is_empty()
    });
    let paths: Vec<&str> = state
        .workspace
        .paths()
        .iter()
        .map(|file| file.path.as_str())
        .collect();
    assert!(paths.contains(&"a.txt"), "the committed file: {paths:?}");
    assert!(
        state.workspace.files.is_empty(),
        "and nothing in it has changed"
    );

    dispatch(
        Cmd::Workspace(workspace::Command::OpenFile {
            at: None,
            path: "a.txt".into(),
        }),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| {
        s.workspace.active().is_some()
    });
    let open = state.workspace.active().expect("the file is open");
    assert_eq!(open.new.text(), "one\n", "its own content, with no change");
}
