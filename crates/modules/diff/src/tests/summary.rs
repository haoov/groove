use std::path::Path;
use std::process::Command;

use groove_types::FileStatus;

use crate::summary;

/// A repository with one commit, and whatever the test writes after it.
pub(crate) fn repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("a temp dir");
    git(dir.path(), &["init", "-q", "-b", "main"]);
    git(dir.path(), &["config", "user.email", "test@example.com"]);
    git(dir.path(), &["config", "user.name", "Test"]);
    write(
        dir.path(),
        "src/lib.rs",
        "fn one() {}\nfn two() {}\nfn three() {}\n",
    );
    write(dir.path(), "README.md", "# title\n");
    git(dir.path(), &["add", "-A"]);
    git(dir.path(), &["commit", "-q", "-m", "first"]);
    dir
}

pub(crate) fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .status()
        .expect("git runs");
    assert!(status.success(), "git {args:?}");
}

pub(crate) fn write(dir: &Path, path: &str, text: &str) {
    let full = dir.join(path);
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent).expect("a directory");
    }
    std::fs::write(full, text).expect("a file");
}

fn files(dir: &Path) -> Vec<(String, u32, u32, FileStatus, bool)> {
    let runtime = tokio::runtime::Runtime::new().expect("a runtime");
    runtime
        .block_on(summary(dir))
        .expect("a summary")
        .into_iter()
        .map(|f| {
            (
                f.path,
                f.added,
                f.deleted,
                f.status,
                f.staged.unwrap_or_default(),
            )
        })
        .collect()
}

#[test]
fn a_clean_worktree_has_nothing_changed() {
    let dir = repo();
    assert!(files(dir.path()).is_empty());
}

#[test]
fn every_kind_of_change_is_listed_with_its_counts() {
    let dir = repo();
    write(
        dir.path(),
        "src/lib.rs",
        "fn one() {}\nfn two() {}\nfn four() {}\nfn five() {}\n",
    );
    write(dir.path(), "src/new.rs", "fn new() {}\nfn other() {}\n");
    std::fs::remove_file(dir.path().join("README.md")).expect("the file goes");
    let files = files(dir.path());
    assert_eq!(
        files,
        vec![
            ("README.md".to_string(), 0, 1, FileStatus::Deleted, false),
            ("src/lib.rs".to_string(), 2, 1, FileStatus::Modified, false),
            ("src/new.rs".to_string(), 2, 0, FileStatus::Untracked, false),
        ],
        "by path, with the counts against HEAD"
    );
}

#[test]
fn a_staged_file_says_so_and_still_counts_the_worktree() {
    let dir = repo();
    write(dir.path(), "src/lib.rs", "fn one() {}\nfn two() {}\n");
    git(dir.path(), &["add", "src/lib.rs"]);
    write(dir.path(), "src/lib.rs", "fn one() {}\n");
    let files = files(dir.path());
    assert_eq!(files.len(), 1);
    let (path, added, deleted, status, staged) = files[0].clone();
    assert_eq!((path.as_str(), added, deleted), ("src/lib.rs", 0, 2));
    assert_eq!(status, FileStatus::Modified);
    assert!(staged, "the index holds part of it");
}

#[test]
fn a_moved_file_is_a_rename_with_the_lines_it_really_changed() {
    let dir = repo();
    git(dir.path(), &["mv", "src/lib.rs", "src/moved.rs"]);
    write(
        dir.path(),
        "src/moved.rs",
        "fn one() {}\nfn two() {}\nfn three() {}\nfn four() {}\n",
    );
    git(dir.path(), &["add", "-A"]);
    let files = files(dir.path());
    assert_eq!(files.len(), 1, "{files:?}");
    let (path, added, deleted, status, _) = files[0].clone();
    assert_eq!(path, "src/moved.rs");
    assert_eq!((added, deleted), (1, 0), "not the whole file");
    assert_eq!(status, FileStatus::Renamed);
}

#[test]
fn a_binary_file_counts_nothing() {
    let dir = repo();
    std::fs::write(dir.path().join("logo.png"), [0u8, 159, 146, 150]).expect("a file");
    git(dir.path(), &["add", "logo.png"]);
    let files = files(dir.path());
    assert_eq!(files.len(), 1);
    assert_eq!((files[0].1, files[0].2), (0, 0));
}

#[test]
fn a_first_commit_that_does_not_exist_yet_still_lists_its_files() {
    let dir = tempfile::tempdir().expect("a temp dir");
    git(dir.path(), &["init", "-q", "-b", "main"]);
    write(dir.path(), "src/lib.rs", "fn one() {}\n");
    let files = files(dir.path());
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].3, FileStatus::Untracked);
    assert_eq!(files[0].1, 1, "an untracked file is all additions");
}

#[tokio::test]
async fn reading_the_worktree_leaves_git_s_own_state_alone() {
    let dir = repo();
    write(dir.path(), "src/lib.rs", "fn one() {}\n");
    let index = dir.path().join(".git/index");
    let before = std::fs::metadata(&index)
        .expect("an index")
        .modified()
        .expect("a time");

    for _ in 0..3 {
        crate::summary(dir.path()).await.expect("a summary");
        crate::opened(dir.path(), "src/lib.rs")
            .await
            .expect("the file");
    }
    let after = std::fs::metadata(&index)
        .expect("an index")
        .modified()
        .expect("a time");
    assert_eq!(
        before, after,
        "a read that writes the index is a watcher that wakes itself"
    );
}
