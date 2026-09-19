use groove_types::Panes;

use crate::panes::{load, path, save};

const LEFT: Panes = Panes {
    rail: 240.0,
    agent: 600.0,
    sidebar: 380.0,
    commit: 120.0,
};

#[test]
fn nothing_was_ever_dragged() {
    let dir = tempfile::tempdir().unwrap();
    assert!(load(&path(dir.path())).unwrap().is_none());
}

#[test]
fn what_one_run_saves_the_next_one_reads() {
    let dir = tempfile::tempdir().unwrap();
    let file = path(dir.path());
    save(&file, &LEFT).unwrap();
    assert_eq!(load(&file).unwrap(), Some(LEFT));
}

#[test]
fn the_file_is_written_under_a_data_dir_that_does_not_exist_yet() {
    let dir = tempfile::tempdir().unwrap();
    let file = path(&dir.path().join("groove"));
    save(&file, &LEFT).unwrap();
    assert!(file.exists());
}
