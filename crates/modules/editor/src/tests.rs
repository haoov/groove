use std::path::Path;

use groove_types::ErrorKind;

use crate::save;

fn worktree() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("a directory");
    std::fs::create_dir(dir.path().join("src")).expect("a subdirectory");
    dir
}

#[test]
fn a_write_lands_under_the_worktree() {
    let dir = worktree();
    save(dir.path(), "src/lib.rs", "fn one() {}\n").expect("the write lands");
    let text = std::fs::read_to_string(dir.path().join("src/lib.rs")).expect("read back");
    assert_eq!(text, "fn one() {}\n");
}

#[test]
fn a_path_leaving_the_worktree_is_refused() {
    let dir = worktree();
    for path in ["../outside.rs", "src/../../outside.rs", "/etc/passwd"] {
        let refused = save(dir.path(), path, "x").expect_err("refused");
        assert_eq!(refused.kind, ErrorKind::Invalid, "{path}");
    }
    assert!(
        !Path::new("/etc/passwd")
            .metadata()
            .is_ok_and(|m| m.len() == 1),
        "nothing outside was written"
    );
}

#[test]
fn a_write_to_a_directory_that_is_not_there_is_an_error_not_a_panic() {
    let dir = worktree();
    let failed = save(dir.path(), "nope/lib.rs", "x").expect_err("an error");
    assert_eq!(failed.kind, ErrorKind::Io);
}

#[test]
fn what_is_written_to_a_clipboard_of_our_own_reads_back() {
    use crate::{Clipboard, Memory};

    let clipboard = Memory::default();
    assert_eq!(clipboard.read(), None, "nothing has been copied");
    clipboard.write("fn one() {}").expect("written");
    assert_eq!(clipboard.read().as_deref(), Some("fn one() {}"));
    clipboard.write("something else").expect("written");
    assert_eq!(clipboard.read().as_deref(), Some("something else"));
}

#[test]
fn a_clipboard_is_shared_across_threads() {
    use crate::{Clipboard, Memory};
    use std::sync::Arc;

    let clipboard = Arc::new(Memory::default());
    let writer = clipboard.clone();
    std::thread::spawn(move || writer.write("from another thread").expect("written"))
        .join()
        .expect("the thread ends");
    assert_eq!(clipboard.read().as_deref(), Some("from another thread"));
}

#[test]
fn a_file_is_made_where_nothing_stands_and_refused_where_something_does() {
    let dir = tempfile::tempdir().unwrap();
    crate::create(dir.path(), "src/one/alpha.rs", false).expect("it is made");
    assert_eq!(
        std::fs::read_to_string(dir.path().join("src/one/alpha.rs")).unwrap(),
        "",
        "and it is empty"
    );
    let again = crate::create(dir.path(), "src/one/alpha.rs", false).expect_err("it is there");
    assert_eq!(again.kind, groove_types::ErrorKind::Conflict);
}

#[test]
fn a_directory_is_made_with_the_ones_above_it() {
    let dir = tempfile::tempdir().unwrap();
    crate::create(dir.path(), "src/deep/down", true).expect("it is made");
    assert!(dir.path().join("src/deep/down").is_dir());
}

#[test]
fn a_path_renames_onto_one_that_is_free() {
    let dir = tempfile::tempdir().unwrap();
    crate::create(dir.path(), "one.rs", false).unwrap();
    crate::rename(dir.path(), "one.rs", "src/two.rs").expect("it moves");
    assert!(!dir.path().join("one.rs").exists());
    assert!(
        dir.path().join("src/two.rs").exists(),
        "and takes its place"
    );

    crate::create(dir.path(), "three.rs", false).unwrap();
    let taken = crate::rename(dir.path(), "three.rs", "src/two.rs").expect_err("it is taken");
    assert_eq!(taken.kind, groove_types::ErrorKind::Conflict);
    let missing = crate::rename(dir.path(), "nothing.rs", "four.rs").expect_err("not there");
    assert_eq!(missing.kind, groove_types::ErrorKind::NotFound);
}

#[test]
fn a_directory_copies_with_everything_under_it() {
    let dir = tempfile::tempdir().unwrap();
    crate::create(dir.path(), "src/one/alpha.rs", false).unwrap();
    crate::save(dir.path(), "src/one/alpha.rs", "one\n").unwrap();
    crate::copy(dir.path(), "src", "copy").expect("it copies");
    assert_eq!(
        std::fs::read_to_string(dir.path().join("copy/one/alpha.rs")).unwrap(),
        "one\n"
    );
    assert!(dir.path().join("src/one/alpha.rs").exists(), "and stays");
}

#[test]
fn a_path_is_taken_away_with_what_stands_under_it() {
    let dir = tempfile::tempdir().unwrap();
    crate::create(dir.path(), "src/one/alpha.rs", false).unwrap();
    crate::delete(dir.path(), "src/one/alpha.rs").expect("the file goes");
    assert!(!dir.path().join("src/one/alpha.rs").exists());
    crate::delete(dir.path(), "src").expect("the directory goes");
    assert!(!dir.path().join("src").exists());
}

#[test]
fn no_operation_reaches_outside_the_worktree() {
    let dir = tempfile::tempdir().unwrap();
    crate::create(dir.path(), "one.rs", false).unwrap();
    for refused in [
        crate::create(dir.path(), "../escaped.rs", false),
        crate::create(dir.path(), "/etc/escaped.rs", false),
        crate::rename(dir.path(), "one.rs", "../escaped.rs"),
        crate::copy(dir.path(), "one.rs", "../escaped.rs"),
        crate::delete(dir.path(), "../one.rs"),
    ] {
        let e = refused.expect_err("it leaves the worktree");
        assert_eq!(e.kind, groove_types::ErrorKind::Invalid, "{e}");
    }
    assert!(
        !dir.path().parent().unwrap().join("escaped.rs").exists(),
        "and nothing was written outside"
    );
}
