//! A file that is no text, on disk or in git: a stand-in on both sides, never a file deleted whole.

use groove_types::RowKind;

use crate::tests::summary::{repo, write};
use crate::{UNSHOWN, changes, opened, summary};

fn kinds_and_texts(dir: &std::path::Path, path: &str) -> (Vec<RowKind>, Vec<String>) {
    let runtime = tokio::runtime::Runtime::new().expect("a runtime");
    let all = runtime.block_on(async {
        let files = summary(dir).await.expect("a summary");
        changes(dir, &files, "HEAD").await.expect("the changes")
    });
    let file = all.get(path).expect("the file changed");
    let kinds = file.rows().map(|row| row.kind).collect();
    (kinds, file.texts().collect())
}

#[test]
fn a_text_file_turned_binary_reads_as_a_stand_in_not_as_deleted() {
    let dir = repo();
    std::fs::write(dir.path().join("README.md"), b"\x89PNG\r\n\x1a\n\0\0\0").unwrap();
    let (kinds, texts) = kinds_and_texts(dir.path(), "README.md");
    assert!(!kinds.contains(&RowKind::Removed), "{kinds:?}");
    assert_eq!(texts, [UNSHOWN.trim_end()]);
}

#[test]
fn a_file_past_the_cap_or_not_utf8_is_not_read_as_text() {
    let dir = repo();
    let big = "x".repeat(crate::READ_MAX_BYTES as usize + 1);
    write(dir.path(), "README.md", &big);
    let (kinds, _) = kinds_and_texts(dir.path(), "README.md");
    assert!(!kinds.contains(&RowKind::Removed), "{kinds:?}");
    std::fs::write(dir.path().join("README.md"), b"caf\xe9\n").unwrap();
    let runtime = tokio::runtime::Runtime::new().expect("a runtime");
    let open = runtime
        .block_on(opened(dir.path(), "README.md", "HEAD"))
        .expect("the file opens");
    assert_eq!(open.new.document().text(), UNSHOWN);
}
