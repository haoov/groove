use std::path::PathBuf;
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

use crate::watch;

const QUIET: Duration = Duration::from_millis(60);
const WAIT: Duration = Duration::from_secs(5);

/// A worktree-shaped temp dir: a `.git`, an ignore rule, source and build output.
fn worktree() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("a temp dir");
    std::fs::create_dir(dir.path().join(".git")).expect("a git dir");
    write(&dir, ".gitignore", "target/\nnode_modules\n");
    write(&dir, "src/lib.rs", "fn one() {}\n");
    for n in 0..5 {
        write(&dir, &format!("target/debug/deps/n{n}/out.o"), "binary");
    }
    write(
        &dir,
        "node_modules/left-pad/index.js",
        "module.exports = 1\n",
    );
    dir
}

fn watching(dir: &tempfile::TempDir) -> (crate::Watch, Receiver<Vec<PathBuf>>) {
    let (sender, batches) = channel();
    let watch = watch(dir.path(), Vec::new(), QUIET, move |paths| {
        let _ = sender.send(paths);
    })
    .expect("a watcher");
    (watch, batches)
}

fn write(dir: &tempfile::TempDir, path: &str, text: &str) {
    let full = dir.path().join(path);
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent).expect("a directory");
    }
    std::fs::write(full, text).expect("a file");
}

#[test]
fn only_the_directories_git_keeps_are_watched() {
    let dir = worktree();
    let (watch, _batches) = watching(&dir);
    assert_eq!(
        watch.watched(),
        2,
        "the root and src; not .git, target or node_modules"
    );
}

#[test]
fn a_write_git_ignores_is_never_reported() {
    let dir = worktree();
    let (_watch, batches) = watching(&dir);
    write(&dir, "target/debug/deps/n0/out.o", "rebuilt");
    write(
        &dir,
        "node_modules/left-pad/index.js",
        "module.exports = 2\n",
    );
    write(&dir, ".git/index", "bookkeeping");
    assert!(
        batches.recv_timeout(QUIET * 8).is_err(),
        "a build and our own git writes must not ask for a read"
    );
    write(&dir, "src/lib.rs", "fn two() {}\n");
    let batch = batches.recv_timeout(WAIT).expect("a notice");
    assert_eq!(batch.len(), 1, "{batch:?}");
    assert!(batch[0].ends_with("lib.rs"));
}

#[test]
fn a_burst_of_writes_costs_fewer_notices_than_writes() {
    let dir = worktree();
    let (_watch, batches) = watching(&dir);
    let writes = 5;
    for n in 0..writes {
        write(&dir, &format!("src/file{n}.rs"), "fn one() {}\n");
    }
    let mut seen: Vec<PathBuf> = Vec::new();
    let mut notices = 0;
    while let Ok(batch) = batches.recv_timeout(QUIET * 8) {
        notices += 1;
        seen.extend(batch);
    }
    assert!(notices > 0, "the burst was reported");
    assert!(notices < writes, "a burst coalesces: {notices} notices");
    for n in 0..writes {
        let name = format!("file{n}.rs");
        assert!(
            seen.iter().any(|path| path.ends_with(&name)),
            "{name} is missing from {seen:?}"
        );
    }
}

#[test]
fn a_directory_that_appears_is_watched_from_then_on() {
    let dir = worktree();
    let (_watch, batches) = watching(&dir);
    std::fs::create_dir(dir.path().join("crates")).expect("a directory");
    let batch = batches
        .recv_timeout(WAIT)
        .expect("the directory is reported");
    assert!(
        batch.iter().any(|path| path.ends_with("crates")),
        "{batch:?}"
    );

    write(&dir, "crates/new.rs", "fn new() {}\n");
    let batch = batches
        .recv_timeout(WAIT)
        .expect("what lands inside it too");
    assert!(
        batch.iter().any(|path| path.ends_with("new.rs")),
        "{batch:?}"
    );
}

#[test]
fn a_directory_that_appears_ignored_stays_unwatched() {
    let dir = worktree();
    let (_watch, batches) = watching(&dir);
    write(&dir, "target/release/out.o", "binary");
    assert!(batches.recv_timeout(QUIET * 8).is_err());
}

#[test]
fn dropping_the_watch_stops_the_notices() {
    let dir = worktree();
    let (watch, batches) = watching(&dir);
    drop(watch);
    write(&dir, "src/lib.rs", "after");
    assert!(batches.recv_timeout(QUIET * 8).is_err());
}

#[test]
fn reading_a_file_is_not_a_change() {
    let dir = worktree();
    let (_watch, batches) = watching(&dir);
    let _ = std::fs::read_to_string(dir.path().join("src/lib.rs")).expect("a read");
    let _ = std::fs::read_to_string(dir.path().join(".gitignore")).expect("a read");
    assert!(
        batches.recv_timeout(QUIET * 8).is_err(),
        "git opens the worktree to answer a read; a notice for that never settles"
    );
}

#[test]
fn an_ignored_directory_that_appears_later_stays_unwatched() {
    let dir = worktree();
    std::fs::remove_dir_all(dir.path().join("target")).expect("no build output yet");
    let (watch, batches) = watching(&dir);
    let before = watch.watched();

    write(&dir, "target/debug/deps/n0/out.o", "binary");
    assert!(
        batches.recv_timeout(QUIET * 8).is_err(),
        "a build starting from nothing must not ask for a read"
    );
    write(&dir, "src/lib.rs", "fn two() {}\n");
    let batch = batches.recv_timeout(WAIT).expect("a notice");
    assert_eq!(batch.len(), 1, "{batch:?}");
    assert!(batch[0].ends_with("lib.rs"));
    assert_eq!(watch.watched(), before, "and the watch count holds");
}

#[test]
fn git_s_own_state_is_watched_and_its_working_noise_is_not() {
    let dir = worktree();
    let git = dir.path().join(".git");
    std::fs::create_dir_all(git.join("refs/heads")).expect("refs");
    std::fs::write(git.join("index"), "i").expect("an index");
    let (sender, batches) = channel();
    let watch = watch(dir.path(), vec![git.clone()], QUIET, move |paths| {
        let _ = sender.send(paths);
    })
    .expect("a watch");

    std::fs::write(git.join("index.lock"), "lock").expect("a lock");
    std::fs::create_dir_all(git.join("objects/ab")).expect("objects");
    std::fs::write(git.join("objects/ab/cdef"), "blob").expect("a loose object");
    std::fs::write(git.join("COMMIT_EDITMSG"), "wip").expect("a message");
    assert!(
        batches.recv_timeout(QUIET * 8).is_err(),
        "git writing its own working files is not a change"
    );

    std::fs::write(git.join("index"), "index again").expect("a stage");
    let batch = batches.recv_timeout(WAIT).expect("the index is a change");
    assert!(
        batch.iter().any(|path| path.ends_with("index")),
        "{batch:?}"
    );

    std::fs::write(git.join("HEAD"), "ref: refs/heads/other\n").expect("a checkout");
    let batch = batches.recv_timeout(WAIT).expect("HEAD is a change");
    assert!(batch.iter().any(|path| path.ends_with("HEAD")), "{batch:?}");
    drop(watch);
}
