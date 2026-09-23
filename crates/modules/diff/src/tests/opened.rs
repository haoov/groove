use groove_types::FileStatus;

use crate::tests::summary::{git, repo, write};
use groove_types::RowKind;

use crate::opened;

fn open(dir: &std::path::Path, path: &str) -> crate::Opened {
    let runtime = tokio::runtime::Runtime::new().expect("a runtime");
    runtime
        .block_on(opened(dir, path, "HEAD"))
        .expect("the file opens")
}

#[test]
fn a_changed_file_carries_both_sides_and_its_rows() {
    let dir = repo();
    write(
        dir.path(),
        "src/lib.rs",
        "fn one() {}\nfn TWO() {}\nfn three() {}\n",
    );
    let file = open(dir.path(), "src/lib.rs");
    assert_eq!(file.old.lines(), 3, "HEAD still has three");
    assert_eq!(file.new.lines(), 3);
    assert!(file.old.is_highlighted() && file.new.document().is_highlighted());
    let kinds: Vec<RowKind> = file.rows.iter().map(|row| row.kind).collect();
    assert_eq!(
        kinds,
        [
            RowKind::Context,
            RowKind::Removed,
            RowKind::Added,
            RowKind::Context
        ]
    );
}

#[test]
fn an_untracked_file_is_all_additions() {
    let dir = repo();
    write(dir.path(), "src/new.rs", "fn new() {}\n");
    let file = open(dir.path(), "src/new.rs");
    assert_eq!(file.old.lines(), 0, "HEAD has no such file");
    assert_eq!(file.rows.len(), 1);
    assert_eq!(file.rows[0].kind, RowKind::Added);
}

#[test]
fn a_deleted_file_is_all_removals() {
    let dir = repo();
    std::fs::remove_file(dir.path().join("README.md")).expect("the file goes");
    let file = open(dir.path(), "README.md");
    assert_eq!(file.new.lines(), 0);
    assert!(file.rows.iter().all(|row| row.kind == RowKind::Removed));
    assert_eq!(file.rows.len(), 1);
}

#[test]
fn a_file_too_long_to_align_says_so_and_keeps_its_text() {
    let dir = repo();
    let line = "fn one() {}\n";
    let text = line.repeat(crate::opened::MAX_SHOWN_BYTES / line.len() + 1);
    write(dir.path(), "src/big.rs", &text);
    let file = open(dir.path(), "src/big.rs");
    assert!(file.long);
    assert!(file.rows.is_empty());
    assert!(file.new.lines() > 0);
}

#[test]
fn a_file_the_worktree_lists_can_always_be_opened() {
    let dir = repo();
    write(dir.path(), "src/lib.rs", "fn one() {}\n");
    write(dir.path(), "notes.txt", "plain\n");
    git(dir.path(), &["add", "notes.txt"]);
    let runtime = tokio::runtime::Runtime::new().expect("a runtime");
    let files = runtime
        .block_on(crate::summary(dir.path()))
        .expect("a summary");
    assert!(!files.is_empty());
    for file in files {
        let opened = open(dir.path(), &file.path);
        let empty = file.status == FileStatus::Deleted && opened.new.lines() == 0;
        assert!(
            !opened.rows.is_empty() || empty,
            "{} has no rows",
            file.path
        );
    }
}

#[test]
fn reopening_with_the_head_side_in_hand_reads_the_same_file() {
    let dir = repo();
    write(
        dir.path(),
        "src/lib.rs",
        "fn one() {}\nfn TWO() {}\nfn three() {}\n",
    );
    let first = open(dir.path(), "src/lib.rs");
    write(
        dir.path(),
        "src/lib.rs",
        "fn one() {}\nfn TWO() {}\nfn THREE() {}\n",
    );
    let again = crate::reopened(dir.path(), "src/lib.rs", first.old.clone());
    let fresh = open(dir.path(), "src/lib.rs");
    assert_eq!(again.rows, fresh.rows);
    assert_eq!(again.marks, fresh.marks);
    assert_eq!(again.old.lines(), fresh.old.lines());
    assert_eq!(again.new.lines(), fresh.new.lines());
    assert!(
        again.new.document().is_highlighted(),
        "the new side is read again"
    );
}
