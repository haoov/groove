use groove_types::{Caret, Edit, FileStatus, Motion};

use crate::tests::fixture::{pooled_clone, services, sh, state, until, worktree};
use crate::{Command as Cmd, Services, SyncSpawner, dispatch, session, workspace};

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

/// The fixture's worktree with `a.txt` open, and where it sits.
fn editing(
    state: &mut crate::AppState,
    services: &Services,
    spawner: &SyncSpawner,
) -> std::path::PathBuf {
    let dir = worktree(state, services, spawner);
    let file = std::path::Path::new(&dir).join("a.txt");
    std::fs::write(&file, "one\ntwo\n").unwrap();
    until(spawner, services, state, |s| !s.workspace.files.is_empty());
    dispatch(
        Cmd::Workspace(workspace::Command::OpenFile {
            path: "a.txt".into(),
        }),
        state,
        services,
        spawner,
    );
    until(spawner, services, state, |s| s.workspace.opened.is_some());
    file
}

fn edit(state: &mut crate::AppState, services: &Services, spawner: &SyncSpawner, edits: &[Edit]) {
    for one in edits {
        dispatch(
            Cmd::Workspace(workspace::Command::Edit(one.clone())),
            state,
            services,
            spawner,
        );
    }
}

fn buffer(state: &crate::AppState) -> String {
    state
        .workspace
        .opened
        .as_ref()
        .map(|open| open.new.text())
        .expect("a file is open")
}

#[test]
fn typing_changes_the_buffer_and_the_rows_follow() {
    let home = tempfile::tempdir().unwrap();
    pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    editing(&mut state, &services, &spawner);
    let rows = state.workspace.opened.as_ref().map(|open| open.rows.len());

    edit(
        &mut state,
        &services,
        &spawner,
        &[
            Edit::Move(Motion::To(Caret::new(1, 3))),
            Edit::Newline,
            Edit::Insert("three".into()),
        ],
    );
    assert_eq!(buffer(&state), "one\ntwo\nthree\n", "the buffer took it");
    assert!(state.workspace.dirty(), "and owes the disk");

    until(&spawner, &services, &mut state, |s| {
        s.workspace.deriving.is_none()
            && s.workspace
                .opened
                .as_ref()
                .is_some_and(|open| Some(open.rows.len()) != rows)
    });
    let open = state.workspace.opened.as_ref().expect("still open");
    assert_eq!(open.new.lines(), 3, "the buffer has the new line");
    assert!(
        open.rows.iter().any(|row| row.new == Some(2)),
        "and the alignment found it: {:?}",
        open.rows
    );
}

#[test]
fn saving_writes_the_buffer_and_clears_what_it_owes() {
    let home = tempfile::tempdir().unwrap();
    pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    let file = editing(&mut state, &services, &spawner);

    edit(
        &mut state,
        &services,
        &spawner,
        &[Edit::Move(Motion::LineEnd), Edit::Insert("!".into())],
    );
    dispatch(
        Cmd::Workspace(workspace::Command::SaveFile),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| !s.workspace.dirty());
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        "one!\ntwo\n",
        "the disk has what the buffer held"
    );
    assert!(state.errors.is_empty(), "{:?}", state.errors);
}

#[test]
fn a_write_on_disk_does_not_take_unsaved_edits_away() {
    let home = tempfile::tempdir().unwrap();
    pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    let file = editing(&mut state, &services, &spawner);

    edit(
        &mut state,
        &services,
        &spawner,
        &[Edit::Move(Motion::LineEnd), Edit::Insert("!".into())],
    );
    std::fs::write(&file, "something else\n").unwrap();
    until(&spawner, &services, &mut state, |s| {
        s.workspace.files.iter().any(|f| f.path == "a.txt")
    });
    assert_eq!(buffer(&state), "one!\ntwo\n", "the buffer is the user's");
    assert!(state.workspace.dirty());
}

#[test]
fn what_is_held_is_copied_cut_and_pasted() {
    let home = tempfile::tempdir().unwrap();
    pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    editing(&mut state, &services, &spawner);

    let hold = [
        Edit::Move(Motion::To(Caret::new(0, 0))),
        Edit::Extend(Motion::LineEnd),
    ];
    edit(&mut state, &services, &spawner, &hold);
    dispatch(
        Cmd::Workspace(workspace::Command::Copy),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |_| {
        services.clipboard.read().is_some()
    });
    assert_eq!(services.clipboard.read().as_deref(), Some("one"));
    assert_eq!(buffer(&state), "one\ntwo\n", "a copy leaves the text alone");

    edit(&mut state, &services, &spawner, &hold);
    dispatch(
        Cmd::Workspace(workspace::Command::Cut),
        &mut state,
        &services,
        &spawner,
    );
    spawner.drain(&mut state, &services);
    assert_eq!(buffer(&state), "\ntwo\n", "a cut takes it out");

    dispatch(
        Cmd::Workspace(workspace::Command::Paste),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| {
        s.workspace
            .opened
            .as_ref()
            .is_some_and(|open| open.new.text() == "one\ntwo\n")
    });
    assert_eq!(buffer(&state), "one\ntwo\n", "and a paste puts it back");
    assert!(state.errors.is_empty(), "{:?}", state.errors);
}

#[test]
fn a_copy_with_nothing_held_says_nothing() {
    let home = tempfile::tempdir().unwrap();
    pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    editing(&mut state, &services, &spawner);
    dispatch(
        Cmd::Workspace(workspace::Command::Copy),
        &mut state,
        &services,
        &spawner,
    );
    spawner.drain(&mut state, &services);
    assert_eq!(
        services.clipboard.read(),
        None,
        "the clipboard is untouched"
    );
}

/// What the summary says is changed, and whether the index holds it.
fn changed(state: &crate::AppState) -> Vec<(String, Option<bool>)> {
    let mut rows: Vec<(String, Option<bool>)> = state
        .workspace
        .files
        .iter()
        .map(|file| (file.path.clone(), file.staged))
        .collect();
    rows.sort();
    rows
}

fn act(
    state: &mut crate::AppState,
    services: &Services,
    spawner: &SyncSpawner,
    command: workspace::Command,
) {
    dispatch(Cmd::Workspace(command), state, services, spawner);
}

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
fn a_new_session_shows_nothing_of_the_one_before_it() {
    let home = tempfile::tempdir().unwrap();
    pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    editing(&mut state, &services, &spawner);
    assert!(state.workspace.opened.is_some(), "a file is open");
    assert!(!state.workspace.files.is_empty());

    dispatch(
        Cmd::Session(session::Command::OpenExplorer { title: None }),
        &mut state,
        &services,
        &spawner,
    );
    assert!(
        state.workspace.opened.is_none(),
        "the file belonged to the worktree that is no longer selected"
    );
    assert!(
        workspace::loaded_for(&state).is_none(),
        "and nothing is loaded for a session with no worktree"
    );
    let selected = state
        .session
        .selected()
        .and_then(|open| open.selected_worktree())
        .map(|w| &w.id);
    assert!(
        state.workspace.files_of(selected).is_empty(),
        "so the sidebar has nothing to list"
    );
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
