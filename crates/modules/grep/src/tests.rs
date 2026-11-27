#[test]
fn every_file_the_ignore_rules_leave_is_listed_from_the_root() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join("src/deep")).unwrap();
    std::fs::create_dir_all(root.join("target")).unwrap();
    std::fs::create_dir_all(root.join(".git")).unwrap();
    std::fs::write(root.join(".gitignore"), "target\n").unwrap();
    std::fs::write(root.join("src/one.rs"), "one\n").unwrap();
    std::fs::write(root.join("src/deep/two.rs"), "two\n").unwrap();
    std::fs::write(root.join("target/built"), "no\n").unwrap();
    std::fs::write(root.join(".git/HEAD"), "no\n").unwrap();

    let found = crate::listed(root, 100);
    assert!(found.contains(&"src/one.rs".to_string()), "{found:?}");
    assert!(found.contains(&"src/deep/two.rs".to_string()), "{found:?}");
    assert!(
        found.contains(&".gitignore".to_string()),
        "hidden files count"
    );
    assert!(
        !found.iter().any(|path| path.starts_with("target")),
        "the ignore rules are kept: {found:?}"
    );
    assert!(
        !found.iter().any(|path| path.starts_with(".git/")),
        "git's own directory is not a file of the worktree: {found:?}"
    );
}

#[test]
fn the_listing_stops_at_the_cap() {
    let dir = tempfile::tempdir().unwrap();
    for at in 0..8 {
        std::fs::write(dir.path().join(format!("file-{at}")), "x\n").unwrap();
    }
    assert_eq!(crate::listed(dir.path(), 3).len(), 3);
}
