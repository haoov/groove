use groove_types::RowKind;

use crate::tests::summary::{repo, write};
use crate::{At, Changes, aligned, changes, summary};

fn all(dir: &std::path::Path) -> Changes {
    let runtime = tokio::runtime::Runtime::new().expect("a runtime");
    runtime.block_on(async {
        let files = summary(dir).await.expect("a summary");
        changes(dir, &files).await
    })
}

#[test]
fn a_file_gives_every_row_its_text() {
    let file = aligned("src/lib.rs", "one\ntwo\n", "one\nTWO\n");
    let kinds: Vec<RowKind> = file.rows.iter().map(|row| row.kind).collect();
    assert_eq!(
        kinds,
        [RowKind::Context, RowKind::Removed, RowKind::Added],
        "{file:?}"
    );
    assert_eq!(file.lines, ["one", "two", "TWO"]);
    assert_eq!(file.indent, 4, "rust indents four");
}

#[test]
fn a_gap_says_how_many_lines_it_hides() {
    let before: String = (0..40).map(|at| format!("line {at}\n")).collect();
    let after = before.replace("line 0\n", "first\n");
    let file = aligned("src/lib.rs", &before, &after);
    let gap = file
        .rows
        .iter()
        .position(|row| matches!(row.kind, RowKind::Gap(_)))
        .expect("a gap");
    assert_eq!(file.lines[gap], "\u{2026} 36 lines");
}

#[test]
fn every_row_of_the_change_belongs_to_a_file() {
    let dir = repo();
    write(dir.path(), "src/lib.rs", "fn one() {}\nfn TWO() {}\n");
    write(dir.path(), "README.md", "# title\n## added\n");
    let changes = all(dir.path());
    assert_eq!(changes.files().len(), 2, "{:?}", changes.files());

    let first = changes.files()[0].path.clone();
    assert_eq!(changes.at(0), Some(At::Head(&changes.files()[0])));
    assert_eq!(changes.head_of(&first), Some(0));
    assert!(matches!(changes.at(1), Some(At::Row(_, 0))));

    let second = changes.head_of(&changes.files()[1].path).expect("the head");
    assert_eq!(second, changes.files()[0].rows.len() + 1);
    assert_eq!(changes.at(second), Some(At::Head(&changes.files()[1])));
    assert_eq!(changes.at(changes.rows()), None, "past the end");
}

#[test]
fn a_file_with_no_head_side_is_all_additions() {
    let dir = repo();
    write(dir.path(), "src/new.rs", "fn new() {}\n");
    let changes = all(dir.path());
    let new = changes.get("src/new.rs").expect("the untracked file");
    assert_eq!(new.rows.len(), 1);
    assert_eq!(new.rows[0].kind, RowKind::Added);
    assert_eq!(new.lines, ["fn new() {}"]);
}
