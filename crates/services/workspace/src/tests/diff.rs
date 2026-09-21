use std::path::PathBuf;

use crate::{State, from_text};

fn showing(path: &str) -> State {
    State {
        opened: Some(from_text(path, "one\n", "two\n")),
        ..State::default()
    }
}

#[test]
fn the_open_file_is_found_among_the_paths_that_changed() {
    let state = showing("src/lib.rs");
    assert!(state.shows(&[PathBuf::from("/w/src/lib.rs")]));
    assert!(state.shows(&[
        PathBuf::from("/w/README.md"),
        PathBuf::from("/w/src/lib.rs"),
    ]));
}

#[test]
fn another_file_of_the_same_name_is_not_the_open_one() {
    let state = showing("src/lib.rs");
    assert!(!state.shows(&[PathBuf::from("/w/lib.rs")]));
    assert!(!state.shows(&[PathBuf::from("/w/other/src/deep/lib.rs")]));
    assert!(!state.shows(&[]));
}

#[test]
fn nothing_open_is_never_shown() {
    assert!(!State::default().shows(&[PathBuf::from("/w/src/lib.rs")]));
}

#[test]
fn a_file_that_stopped_changing_stays_open() {
    let mut state = showing("src/lib.rs");
    state.loaded(
        groove_types::WorktreeId::new("w1"),
        Vec::new(),
        crate::Changes::default(),
    );
    assert!(state.files.is_empty(), "nothing changed any more");
    assert!(
        state.opened.is_some(),
        "the editor keeps the file it was showing"
    );
}

#[test]
fn a_file_taken_off_the_disk_is_shut() {
    let mut state = showing("src/lib.rs");
    state.shut_if_gone("src/other.rs");
    assert!(state.opened.is_some(), "another path is not this one");
    state.shut_if_gone("src/lib.rs");
    assert!(state.opened.is_none());
}

#[test]
fn a_file_under_a_deleted_directory_is_shut() {
    let mut state = showing("src/deep/lib.rs");
    state.shut_if_gone("src/dee");
    assert!(state.opened.is_some(), "a name it only starts with");
    state.shut_if_gone("src/deep");
    assert!(state.opened.is_none());
}
