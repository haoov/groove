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
