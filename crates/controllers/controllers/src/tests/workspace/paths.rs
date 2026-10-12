//! One path made, moved, copied or taken away, and the worktree read again after it.

use groove_workspace_service::PathOp;

use super::*;

fn act(state: &mut crate::AppState, services: &Services, spawner: &SyncSpawner, op: PathOp) {
    send(state, services, spawner, workspace::Command::Path(op));
}

#[test]
fn a_file_made_in_the_worktree_shows_up_as_a_change() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    let dir = worktree(&mut state, &services, &spawner);

    act(
        &mut state,
        &services,
        &spawner,
        PathOp::Create {
            path: "src/two.rs".into(),
            folder: false,
        },
    );
    assert!(state.errors.is_empty(), "{:?}", state.errors);
    assert!(std::path::Path::new(&dir).join("src/two.rs").exists());
    until(&spawner, &services, &mut state, |s| {
        s.workspace
            .files
            .iter()
            .any(|file| file.path == "src/two.rs")
    });
    assert!(
        state.workspace.paths().is_empty(),
        "the walk is dropped, so the tree reads the worktree again"
    );
}

#[test]
fn a_path_renamed_leaves_its_old_name_behind() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    let dir = worktree(&mut state, &services, &spawner);
    let at = std::path::Path::new(&dir);

    act(
        &mut state,
        &services,
        &spawner,
        PathOp::Rename {
            from: "a.txt".into(),
            to: "b.txt".into(),
        },
    );
    assert!(state.errors.is_empty(), "{:?}", state.errors);
    assert!(!at.join("a.txt").exists());
    assert!(at.join("b.txt").exists());
}

#[test]
fn a_path_that_leaves_the_worktree_is_refused_and_says_so() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    worktree(&mut state, &services, &spawner);

    act(
        &mut state,
        &services,
        &spawner,
        PathOp::Create {
            path: "../escaped.rs".into(),
            folder: false,
        },
    );
    assert_eq!(state.errors.len(), 1, "{:?}", state.errors);
    assert_eq!(state.errors[0].kind, groove_types::ErrorKind::Invalid);
    assert!(
        !home.path().join("escaped.rs").exists(),
        "and nothing was written"
    );
}

#[test]
fn a_directory_and_what_it_holds_go_together() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    let dir = worktree(&mut state, &services, &spawner);
    let at = std::path::Path::new(&dir);

    act(
        &mut state,
        &services,
        &spawner,
        PathOp::Create {
            path: "deep/down/one.rs".into(),
            folder: false,
        },
    );
    act(
        &mut state,
        &services,
        &spawner,
        PathOp::Copy {
            from: "deep".into(),
            to: "copied".into(),
        },
    );
    assert!(
        at.join("copied/down/one.rs").exists(),
        "everything under it"
    );

    act(
        &mut state,
        &services,
        &spawner,
        PathOp::Delete {
            path: "deep".into(),
        },
    );
    assert!(!at.join("deep").exists());
    assert!(at.join("copied/down/one.rs").exists(), "the copy stays");
    assert!(state.errors.is_empty(), "{:?}", state.errors);
}

#[test]
fn a_rename_then_a_delete_of_its_new_name_run_in_that_order() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    let dir = worktree(&mut state, &services, &spawner);
    let at = std::path::Path::new(&dir);
    let held = crate::tests::fixture::Held::over(&spawner);
    let rename = PathOp::Rename {
        from: "a.txt".into(),
        to: "b.txt".into(),
    };
    let delete = PathOp::Delete {
        path: "b.txt".into(),
    };
    for op in [rename, delete] {
        let path = Cmd::Workspace(workspace::Command::Path(op));
        dispatch(path, &mut state, &services, &held);
    }
    held.backwards(&mut state, &services);
    assert!(state.errors.is_empty(), "{:?}", state.errors);
    assert!(!at.join("a.txt").exists() && !at.join("b.txt").exists());
}

#[test]
fn two_saves_reach_the_disk_in_the_order_asked() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    let file = editing(&mut state, &services, &spawner);
    let held = crate::tests::fixture::Held::over(&spawner);
    for mark in ["!", "?"] {
        let typed = [Edit::Move(Motion::LineEnd), Edit::Insert(mark.into())];
        edit(&mut state, &services, &spawner, &typed);
        let save = Cmd::Workspace(workspace::Command::SaveFile);
        dispatch(save, &mut state, &services, &held);
    }
    held.backwards(&mut state, &services);
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "one!?\ntwo\n");
    assert!(
        state.workspace.dirty().is_empty(),
        "and the buffer owes nothing"
    );
}
