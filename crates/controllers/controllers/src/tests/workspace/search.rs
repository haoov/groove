//! A search across the worktree, from the command to what lands in the state.

use super::*;

#[test]
fn a_search_across_the_worktree_reports_what_it_finds() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    let dir = worktree(&mut state, &services, &spawner);
    let root = std::path::Path::new(&dir);
    std::fs::write(root.join("a.txt"), "one needle here\nplain\n").unwrap();
    std::fs::create_dir_all(root.join("deep")).unwrap();
    std::fs::write(root.join("deep/b.txt"), "another NEEDLE\n").unwrap();

    dispatch(
        Cmd::Workspace(workspace::Command::Grep {
            query: "needle".into(),
            under: String::new(),
        }),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| {
        s.workspace.found.len() == 2
    });
    let mut found: Vec<(String, usize)> = state
        .workspace
        .found
        .iter()
        .map(|one| (one.path.clone(), one.line))
        .collect();
    found.sort();
    assert_eq!(
        found,
        [("a.txt".to_string(), 0), ("deep/b.txt".to_string(), 0)],
        "both cases, at any depth"
    );

    dispatch(
        Cmd::Workspace(workspace::Command::Grep {
            query: String::new(),
            under: String::new(),
        }),
        &mut state,
        &services,
        &spawner,
    );
    assert!(
        state.workspace.found.is_empty(),
        "an empty query finds nothing"
    );
}

#[test]
fn a_search_looks_only_where_the_path_lets_it() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    let dir = worktree(&mut state, &services, &spawner);
    let root = std::path::Path::new(&dir);
    std::fs::write(root.join("a.txt"), "one needle here\n").unwrap();
    std::fs::create_dir_all(root.join("deep")).unwrap();
    std::fs::write(root.join("deep/b.txt"), "another needle\n").unwrap();

    dispatch(
        Cmd::Workspace(workspace::Command::Grep {
            query: "needle".into(),
            under: "deep".into(),
        }),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| {
        s.workspace.searching.is_none()
    });
    let paths: Vec<&str> = state
        .workspace
        .found
        .iter()
        .map(|one| one.path.as_str())
        .collect();
    assert_eq!(paths, ["deep/b.txt"], "the other file was never read");
}

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
        !s.workspace.paths.is_empty()
    });
    let paths: Vec<&str> = state
        .workspace
        .paths
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
        s.workspace.opened.is_some()
    });
    let open = state.workspace.opened.as_ref().expect("the file is open");
    assert_eq!(open.new.text(), "one\n", "its own content, with no change");
}
