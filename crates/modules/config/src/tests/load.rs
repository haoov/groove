use groove_types::{ErrorKind, ThemeName};

use crate::{Error, load, path};

const FILE: &str = r#"{ "git": { "worktree_root": "~/worktrees" }, "ui": { "theme": "mocha" } }"#;

#[test]
fn the_path_is_the_file_under_the_config_dir() {
    assert_eq!(
        path(std::path::Path::new("/home/x/.config/groove")),
        std::path::PathBuf::from("/home/x/.config/groove/config.json")
    );
}

#[test]
fn a_missing_file_is_first_run() {
    let dir = tempfile::tempdir().unwrap();
    assert!(load(&path(dir.path())).unwrap().is_none());
}

#[test]
fn a_file_loads_with_its_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let file = path(dir.path());
    std::fs::write(&file, FILE).unwrap();
    let config = load(&file).unwrap().unwrap();
    assert_eq!(config.git.worktree_root, "~/worktrees");
    assert_eq!(config.ui.theme, ThemeName::Mocha);
    assert_eq!(config.ui.font_size, 13.0, "the design's own size");
    assert_eq!(config.preferences.poll_interval_secs, 60);
}

#[test]
fn a_broken_file_names_itself() {
    let dir = tempfile::tempdir().unwrap();
    let file = path(dir.path());
    std::fs::write(&file, "{ not json").unwrap();
    let err = load(&file).unwrap_err();
    assert!(matches!(err, Error::Parse { .. }));
    let message = err.to_string();
    assert!(message.contains("config.json"), "{message}");
    assert_eq!(groove_types::Error::from(err).kind, ErrorKind::Invalid);
}

#[test]
fn a_saved_config_loads_back_as_it_was_written() {
    let dir = tempfile::tempdir().unwrap();
    let file = path(&dir.path().join("groove"));
    std::fs::write(dir.path().join("seed.json"), FILE).unwrap();
    let mut config = load(&dir.path().join("seed.json")).unwrap().unwrap();
    config.preferences.poll_interval_secs = 90;
    crate::save(&file, &config).unwrap();
    assert_eq!(load(&file).unwrap(), Some(config));
}
