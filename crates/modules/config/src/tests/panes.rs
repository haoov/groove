use groove_types::Panes;

use crate::panes::{load, path, save};

const LEFT: Panes = Panes {
    rail: 240.0,
    agent: 600.0,
    sidebar: 380.0,
    commit: 120.0,
    band: 300.0,
    feed: 200.0,
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
fn a_file_from_before_the_timeline_opens_the_band_at_its_own_height() {
    let dir = tempfile::tempdir().unwrap();
    let file = path(dir.path());
    std::fs::create_dir_all(file.parent().expect("a parent")).unwrap();
    let older = r#"{"rail":240.0,"agent":600.0,"sidebar":380.0,"commit":120.0}"#;
    std::fs::write(&file, older).unwrap();
    let read = load(&file).unwrap().expect("the older file");
    assert_eq!(read.rail, 240.0);
    assert_eq!(read.band, 260.0, "the band's own default");
}

#[test]
fn the_file_is_written_under_a_data_dir_that_does_not_exist_yet() {
    let dir = tempfile::tempdir().unwrap();
    let file = path(&dir.path().join("groove"));
    save(&file, &LEFT).unwrap();
    assert!(file.exists());
}
