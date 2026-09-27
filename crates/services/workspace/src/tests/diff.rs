use std::path::PathBuf;

use groove_types::WorktreeId;

use crate::{Changes, State, from_text};

fn showing(path: &str) -> State {
    let mut state = State::default();
    selected(&mut state, "w1");
    state.arrived(
        &WorktreeId::new("w1"),
        from_text(path, "one\n", "two\n"),
        None,
        true,
    );
    state
}

fn selected(state: &mut State, worktree: &str) {
    state.loaded(WorktreeId::new(worktree), Vec::new(), Changes::default());
}

#[test]
fn the_open_file_is_found_among_the_paths_that_changed() {
    let state = showing("src/lib.rs");
    assert_eq!(
        state.shows(&[PathBuf::from("/w/src/lib.rs")]),
        ["src/lib.rs"]
    );
    let both = [
        PathBuf::from("/w/README.md"),
        PathBuf::from("/w/src/lib.rs"),
    ];
    assert_eq!(state.shows(&both), ["src/lib.rs"]);
}

#[test]
fn another_file_of_the_same_name_is_not_the_open_one() {
    let state = showing("src/lib.rs");
    assert!(state.shows(&[PathBuf::from("/w/lib.rs")]).is_empty());
    assert!(
        state
            .shows(&[PathBuf::from("/w/other/src/deep/lib.rs")])
            .is_empty()
    );
    assert!(state.shows(&[]).is_empty());
}

#[test]
fn nothing_open_is_never_shown() {
    assert!(
        State::default()
            .shows(&[PathBuf::from("/w/src/lib.rs")])
            .is_empty()
    );
}

#[test]
fn a_file_that_stopped_changing_stays_open() {
    let mut state = showing("src/lib.rs");
    selected(&mut state, "w1");
    assert!(state.files.is_empty(), "nothing changed any more");
    assert!(
        state.active().is_some(),
        "the editor keeps the file it was showing"
    );
}

#[test]
fn a_worktree_s_open_files_wait_while_another_is_selected() {
    let mut state = showing("src/lib.rs");
    selected(&mut state, "w2");
    assert!(
        state.active().is_none(),
        "the other worktree has its own files"
    );
    selected(&mut state, "w1");
    let back = state.active().map(|open| open.path.as_str());
    assert_eq!(back, Some("src/lib.rs"));
}

#[test]
fn a_file_taken_off_the_disk_is_shut() {
    let mut state = showing("src/lib.rs");
    state.shut_if_gone("src/other.rs");
    assert!(state.active().is_some(), "another path is not this one");
    state.shut_if_gone("src/lib.rs");
    assert!(state.active().is_none());
}

#[test]
fn a_file_under_a_deleted_directory_is_shut() {
    let mut state = showing("src/deep/lib.rs");
    state.shut_if_gone("src/dee");
    assert!(state.active().is_some(), "a name it only starts with");
    state.shut_if_gone("src/deep");
    assert!(state.active().is_none());
}
