#![allow(clippy::print_stdout)]

use std::sync::Arc;

use crate::{Search, walk};

fn tree() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("a temp dir");
    std::fs::create_dir_all(dir.path().join("src")).expect("a directory");
    std::fs::write(
        dir.path().join("src/one.rs"),
        "let value = 1;\nlet other = 2;\n",
    )
    .unwrap();
    std::fs::write(dir.path().join("src/two.rs"), "let VALUE = 3;\n").unwrap();
    std::fs::write(dir.path().join(".gitignore"), "skip/\n").unwrap();
    std::fs::create_dir_all(dir.path().join("skip")).expect("a directory");
    std::fs::write(dir.path().join("skip/three.rs"), "let value = 4;\n").unwrap();
    dir
}

fn all(dir: &std::path::Path, query: &str, cap: usize) -> Vec<crate::Found> {
    under(dir, query, "", cap)
}

fn under(dir: &std::path::Path, query: &str, under: &str, cap: usize) -> Vec<crate::Found> {
    let mut found = Vec::new();
    walk(
        dir,
        query,
        under,
        cap,
        Arc::new(Search::default()),
        |batch| found.extend(batch),
    );
    found.sort_by_key(|one| (one.path.clone(), one.line));
    found
}

#[test]
fn it_reads_every_file_the_ignore_rules_leave() {
    let dir = tree();
    let found = all(dir.path(), "value", 100);
    let where_: Vec<(&str, usize)> = found
        .iter()
        .map(|one| (one.path.as_str(), one.line))
        .collect();
    assert_eq!(
        where_,
        [("src/one.rs", 0), ("src/two.rs", 0)],
        "both cases, and nothing under an ignored directory"
    );
    assert_eq!(found[0].at, (4, 9), "where the word sits in the line");
    assert_eq!(found[0].text, "let value = 1;");
}

#[test]
fn a_search_stops_when_it_has_enough() {
    let dir = tree();
    assert_eq!(all(dir.path(), "let", 1).len(), 1, "the cap is honoured");
}

#[test]
fn a_stopped_search_gives_nothing_back() {
    let dir = tree();
    let search = Arc::new(Search::default());
    search.stop();
    let mut found = Vec::new();
    walk(dir.path(), "value", "", 100, search, |batch| {
        found.extend(batch)
    });
    assert!(found.is_empty());
}

#[test]
#[ignore]
fn time_a_walk_of_the_repository() {
    let root = std::path::Path::new("../../..");
    for (query, cap) in [("workspace", 500), ("Aligned", 500), ("zzz_nothing", 500)] {
        let started = std::time::Instant::now();
        let mut count = 0;
        walk(root, query, "", cap, Arc::new(Search::default()), |batch| {
            count += batch.len()
        });
        println!(
            "{query:>12}: {:>10?} for {count} matches",
            started.elapsed()
        );
    }
}

#[test]
fn a_path_narrows_which_files_are_read() {
    let dir = tree();
    let found = under(dir.path(), "value", "two", 100);
    let paths: Vec<&str> = found.iter().map(|one| one.path.as_str()).collect();
    assert_eq!(paths, ["src/two.rs"], "only the files the path keeps");
    assert!(
        under(dir.path(), "value", "nowhere", 100).is_empty(),
        "a path nothing matches finds nothing"
    );
}

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

    let found = crate::paths(root, 100);
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
    assert_eq!(crate::paths(dir.path(), 3).len(), 3);
}
