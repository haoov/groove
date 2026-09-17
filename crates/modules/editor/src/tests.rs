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
