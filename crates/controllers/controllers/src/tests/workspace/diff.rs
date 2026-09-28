//! Reading a worktree: what changed, what the rows show, what a mark remembers.

use super::*;

#[test]
fn load_lists_what_changed_in_the_selected_worktree() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    let dir = worktree(&mut state, &services, &spawner);
    assert!(
        state.workspace.worktree.as_ref().is_some(),
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
    let (_home, spawner, services, mut state) = crate::tests::fixture::fresh();
    dispatch(
        Cmd::Workspace(workspace::Command::Load),
        &mut state,
        &services,
        &spawner,
    );
    spawner.drain(&mut state, &services);
    assert!(state.workspace.worktree.as_ref().is_none());
    assert!(!workspace::stale(&state), "nothing to load is not stale");
}

#[test]
fn switching_to_a_session_with_no_worktree_forgets_the_last_one() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
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
        state.workspace.worktree.as_ref().is_none(),
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
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
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
fn a_new_session_shows_nothing_of_the_one_before_it() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    editing(&mut state, &services, &spawner);
    assert!(state.workspace.active().is_some(), "a file is open");
    assert!(!state.workspace.files.is_empty());

    dispatch(
        Cmd::Session(session::Command::OpenExplorer { title: None }),
        &mut state,
        &services,
        &spawner,
    );
    assert!(
        state.workspace.active().is_none(),
        "the file belonged to the worktree that is no longer selected"
    );
    assert!(
        state.workspace.worktree.as_ref().is_none(),
        "and nothing is loaded for a session with no worktree"
    );
    let selected = state.session.selected_worktree().map(|w| &w.id);
    assert!(
        state.workspace.files_of(selected).is_empty(),
        "so the sidebar has nothing to list"
    );
}

#[test]
fn the_change_is_one_surface_and_the_rows_on_screen_take_their_colours() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    let dir = worktree(&mut state, &services, &spawner);
    std::fs::write(std::path::Path::new(&dir).join("a.txt"), "two\n").unwrap();
    std::fs::write(std::path::Path::new(&dir).join("b.rs"), "fn b() {}\n").unwrap();
    until(&spawner, &services, &mut state, |s| {
        s.workspace.changes.files().len() == 2
    });
    let changes = &state.workspace.changes;
    assert_eq!(
        changes.rows(),
        changes.files().iter().map(|f| f.len() + 1).sum::<usize>()
    );
    assert_eq!(changes.head_of("b.rs"), Some(changes.files()[0].len() + 1));
    assert!(
        state.workspace.coloured.is_empty(),
        "nothing is on screen yet"
    );

    dispatch(
        Cmd::Workspace(workspace::Command::Show {
            rows: 0..changes.rows(),
        }),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| {
        s.workspace.coloured.len() == 2
    });
    let painted = state.workspace.coloured.get("b.rs").expect("the rust file");
    assert!(painted.new.is_highlighted(), "its grammar was read");

    dispatch(
        Cmd::Workspace(workspace::Command::Show { rows: 0..1 }),
        &mut state,
        &services,
        &spawner,
    );
    assert_eq!(
        state.workspace.coloured.keys().collect::<Vec<_>>(),
        ["a.txt", "b.rs"],
        "a file just off screen keeps its documents, ready for the row it takes"
    );
}

#[test]
fn a_file_is_read_before_its_rows_are_on_screen() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    let dir = worktree(&mut state, &services, &spawner);
    std::fs::write(
        std::path::Path::new(&dir).join("a.txt"),
        "two
",
    )
    .unwrap();
    std::fs::write(
        std::path::Path::new(&dir).join("b.rs"),
        "fn b() {}
",
    )
    .unwrap();
    until(&spawner, &services, &mut state, |s| {
        s.workspace.changes.files().len() == 2
    });

    dispatch(
        Cmd::Workspace(workspace::Command::Show { rows: 0..1 }),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| {
        s.workspace.coloured.contains_key("b.rs")
    });
    assert!(
        state.workspace.sides("b.rs").is_some(),
        "the file has its colours before a row of it is drawn"
    );
}

#[test]
fn a_keystroke_reaches_the_rows_the_whole_change_shows() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    editing(&mut state, &services, &spawner);
    dispatch(
        Cmd::Workspace(workspace::Command::Edit(Edit::Insert("X".into()))),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| {
        s.workspace
            .changes
            .get("a.txt")
            .is_some_and(|file| file.texts().any(|line| line.starts_with('X')))
    });
}

#[test]
fn a_file_marked_read_is_remembered_by_the_session() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    let dir = worktree(&mut state, &services, &spawner);
    std::fs::write(std::path::Path::new(&dir).join("a.txt"), "two\n").unwrap();
    until(&spawner, &services, &mut state, |s| {
        !s.workspace.files.is_empty()
    });
    let id = state.session.selected.clone().expect("a session");
    let picked = state
        .session
        .selected_worktree()
        .map(|worktree| worktree.id.clone())
        .expect("a worktree");

    dispatch(
        Cmd::Workspace(workspace::Command::MarkRead {
            path: "a.txt".into(),
        }),
        &mut state,
        &services,
        &spawner,
    );
    assert!(
        state
            .session
            .selected()
            .expect("the row")
            .is_read(&picked, "a.txt"),
        "the session says so at once"
    );
    until(&spawner, &services, &mut state, |_| true);
    let kept = spawner
        .block_on(services.session.contents(&id))
        .expect("the contents");
    assert_eq!(
        kept.read,
        [(picked.clone(), "a.txt".to_string())],
        "and the disk holds it"
    );

    dispatch(
        Cmd::Workspace(workspace::Command::MarkRead {
            path: "a.txt".into(),
        }),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |_| true);
    let gone = spawner
        .block_on(services.session.contents(&id))
        .expect("the contents");
    assert!(gone.read.is_empty(), "marked again takes it off");
}

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

/// The branch the selected worktree merges into, as a task's own would carry it.
pub(super) fn based(state: &mut crate::AppState, branch: &str) {
    let id = state.session.selected.clone().expect("a session");
    let open = state.session.get_mut(&id).expect("the session");
    for worktree in &mut open.worktrees {
        worktree.base_ref = Some(branch.to_string());
    }
}

#[test]
fn an_answer_read_for_another_selection_is_dropped() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    let dir = worktree(&mut state, &services, &spawner);
    std::fs::write(std::path::Path::new(&dir).join("a.txt"), "two\n").unwrap();
    dispatch(
        Cmd::Workspace(workspace::Command::Load),
        &mut state,
        &services,
        &spawner,
    );
    state.workspace.mode = groove_types::DiffMode::Base;
    spawner.drain(&mut state, &services);
    assert!(
        state.workspace.files.is_empty(),
        "read in the working mode, landed in the base one"
    );
}
