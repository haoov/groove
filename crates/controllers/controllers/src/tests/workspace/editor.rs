//! The open file through the controller: what a keystroke, a save and a write do.

use super::*;

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

#[test]
fn a_file_opened_at_a_match_holds_it() {
    let home = tempfile::tempdir().unwrap();
    pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    let dir = worktree(&mut state, &services, &spawner);
    std::fs::write(std::path::Path::new(&dir).join("a.txt"), "one two\nthree\n").unwrap();
    until(&spawner, &services, &mut state, |s| {
        !s.workspace.files.is_empty()
    });
    let held = groove_types::Selection {
        anchor: groove_types::Caret::new(0, 4),
        head: groove_types::Caret::new(0, 7),
    };
    dispatch(
        Cmd::Workspace(workspace::Command::OpenFile {
            path: "a.txt".into(),
            at: Some(held),
        }),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| {
        s.workspace.opened.is_some()
    });
    let open = state.workspace.opened.as_ref().expect("the file");
    assert_eq!(open.new.selected(), "two", "the match, held by the caret");
}

#[test]
fn an_undo_after_a_save_takes_back_what_was_typed() {
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
    act(
        &mut state,
        &services,
        &spawner,
        workspace::Command::SaveFile,
    );
    until(&spawner, &services, &mut state, |s| !s.workspace.dirty());

    let stamp = state.workspace.stamp;
    std::fs::write(&file, "one!\ntwo\n").unwrap();
    until(&spawner, &services, &mut state, |s| {
        s.workspace.stamp != stamp
    });
    assert_eq!(
        buffer(&state),
        "one!\ntwo\n",
        "the read found the same text"
    );

    edit(&mut state, &services, &spawner, &[Edit::Undo]);
    assert_eq!(buffer(&state), "one\ntwo\n", "the buffer kept its history");
    assert!(state.workspace.dirty(), "and owes the disk again");
}
