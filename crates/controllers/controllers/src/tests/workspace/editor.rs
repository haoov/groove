//! The open file through the controller: what a keystroke, a save and a write do.

use super::*;

#[test]
fn the_open_file_follows_a_change_on_disk() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
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
        s.workspace.active().is_some()
    });
    let rows = state
        .workspace
        .active()
        .map(|open| open.hunked.layout.len());
    assert_eq!(rows, Some(2), "one line out, one line in");

    std::fs::write(&file, "two\nthree\nfour\n").unwrap();
    until(&spawner, &services, &mut state, |s| {
        s.workspace
            .active()
            .is_some_and(|open| open.new.lines() == 3)
    });
    let open = state.workspace.active().expect("still open");
    assert_eq!(open.path, "a.txt", "the same file, read again");
    assert_eq!(open.hunked.layout.len(), 4, "one out, three in");
}

#[test]
fn an_open_file_changed_while_away_is_read_again_on_return() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    let file = editing(&mut state, &services, &spawner);

    state.workspace.clear();
    std::fs::write(&file, "one\ntwo\nthree\n").unwrap();
    workspace::follow(&mut state, &spawner);
    until(&spawner, &services, &mut state, |s| {
        s.workspace
            .active()
            .is_some_and(|open| open.new.lines() == 3)
    });
    let (_, new) = state.workspace.sides("a.txt").expect("both sides");
    assert_eq!(new.text(), "one\ntwo\nthree\n", "the diff reads the disk");
}

#[test]
fn typing_changes_the_buffer_and_the_rows_follow() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    editing(&mut state, &services, &spawner);
    let rows = state
        .workspace
        .active()
        .map(|open| open.hunked.layout.len());

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
    assert!(!state.workspace.dirty().is_empty(), "and owes the disk");

    until(&spawner, &services, &mut state, |s| {
        s.workspace.deriving.is_empty()
            && s.workspace
                .active()
                .is_some_and(|open| Some(open.hunked.layout.len()) != rows)
    });
    let open = state.workspace.active().expect("still open");
    assert_eq!(open.new.lines(), 3, "the buffer has the new line");
    assert!(
        open.hunked.layout.all().any(|row| row.new == Some(2)),
        "and the alignment found it: {:?}",
        open.hunked.hunks
    );
}

#[test]
fn saving_writes_the_buffer_and_clears_what_it_owes() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
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
    until(&spawner, &services, &mut state, |s| {
        s.workspace.dirty().is_empty()
    });
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        "one!\ntwo\n",
        "the disk has what the buffer held"
    );
    assert!(state.errors.is_empty(), "{:?}", state.errors);
}

#[test]
fn a_write_on_disk_does_not_take_unsaved_edits_away() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
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
    assert!(!state.workspace.dirty().is_empty());
}

#[test]
fn what_is_held_is_copied_cut_and_pasted() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
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
            .active()
            .is_some_and(|open| open.new.text() == "one\ntwo\n")
    });
    assert_eq!(buffer(&state), "one\ntwo\n", "and a paste puts it back");
    assert!(state.errors.is_empty(), "{:?}", state.errors);
}

#[test]
fn a_copy_with_nothing_held_says_nothing() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
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
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
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
        s.workspace.active().is_some()
    });
    let open = state.workspace.active().expect("the file");
    assert_eq!(open.new.selected(), "two", "the match, held by the caret");
}

#[test]
fn an_undo_after_a_save_takes_back_what_was_typed() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
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
    until(&spawner, &services, &mut state, |s| {
        s.workspace.dirty().is_empty()
    });

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
    assert!(
        !state.workspace.dirty().is_empty(),
        "and owes the disk again"
    );
}

#[test]
fn a_file_saved_back_to_what_it_was_stays_open() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    editing(&mut state, &services, &spawner);
    assert_eq!(buffer(&state), "one\ntwo\n");

    edit(
        &mut state,
        &services,
        &spawner,
        &[
            Edit::Move(Motion::To(Caret::new(1, 0))),
            Edit::SelectLine,
            Edit::Delete,
            Edit::Backspace,
        ],
    );
    assert_eq!(buffer(&state), "one\n", "back to what the commit holds");
    send(
        &mut state,
        &services,
        &spawner,
        workspace::Command::SaveFile,
    );
    send(&mut state, &services, &spawner, workspace::Command::Load);
    assert!(
        state.workspace.files.is_empty(),
        "nothing is changed any more: {:?}",
        changed(&state)
    );
    assert!(
        state.workspace.active().is_some(),
        "and the file is still the open one"
    );
}

/// One command, run to the end of everything it starts.
fn send(
    state: &mut crate::AppState,
    services: &Services,
    spawner: &SyncSpawner,
    command: workspace::Command,
) {
    dispatch(Cmd::Workspace(command), state, services, spawner);
    until(spawner, services, state, |s| s.pending.is_empty());
}

fn keyed(state: &mut crate::AppState, services: &Services, spawner: &SyncSpawner, edit: Edit) {
    let command = Cmd::Workspace(workspace::Command::Edit(edit));
    dispatch(command, state, services, spawner);
}

#[test]
fn a_newline_stands_as_a_row_before_the_diff_runs_again() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    editing(&mut state, &services, &spawner);
    until(&spawner, &services, &mut state, |s| {
        s.workspace.deriving.is_empty()
    });
    let (stream, rows) = (
        state.workspace.changes.rows(),
        state
            .workspace
            .active()
            .map(|open| open.hunked.layout.len()),
    );

    let end = Edit::Move(Motion::To(Caret::new(1, 3)));
    keyed(&mut state, &services, &spawner, end);
    keyed(&mut state, &services, &spawner, Edit::Newline);
    let open = state.workspace.active().expect("still open");
    assert_eq!(
        state.workspace.changes.rows(),
        stream + 1,
        "the stream has its row"
    );
    assert_eq!(Some(open.hunked.layout.len()), rows.map(|one| one + 1));
    assert!(open.hunked.marks.contains_key(&2), "and marks the new line");
}

#[test]
fn an_edit_undone_leaves_the_hunks_as_they_stood() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    editing(&mut state, &services, &spawner);
    until(&spawner, &services, &mut state, |s| {
        s.workspace.deriving.is_empty()
    });
    let hunks = |s: &crate::AppState| s.workspace.active().map(|open| open.hunked.hunks.clone());
    let before = hunks(&state);

    let start = Edit::Move(Motion::To(Caret::new(0, 0)));
    keyed(&mut state, &services, &spawner, start);
    keyed(&mut state, &services, &spawner, Edit::Insert("x".into()));
    assert_ne!(hunks(&state), before, "the typed line is a change");
    keyed(&mut state, &services, &spawner, Edit::Undo);
    assert_eq!(hunks(&state), before, "and is none once taken back");
}

#[test]
fn a_newline_after_a_line_leaves_that_line_unchanged() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    editing(&mut state, &services, &spawner);
    until(&spawner, &services, &mut state, |s| {
        s.workspace.deriving.is_empty()
    });

    let end = Edit::Move(Motion::To(Caret::new(0, 3)));
    keyed(&mut state, &services, &spawner, end);
    keyed(&mut state, &services, &spawner, Edit::Newline);
    let open = state.workspace.active().expect("still open");
    let gone = open
        .hunked
        .layout
        .all()
        .any(|row| row.kind == groove_types::RowKind::Removed);
    assert!(!gone, "nothing reads as taken out: {:?}", open.hunked.hunks);
    assert!(
        !open.hunked.marks.contains_key(&0),
        "the line it ended stays as it was"
    );
    assert!(
        open.hunked.marks.contains_key(&1),
        "the line it made is new"
    );
}
