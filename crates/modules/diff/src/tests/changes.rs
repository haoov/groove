use groove_types::RowKind;

use crate::tests::summary::{repo, write};
use crate::{At, Changes, aligned, changes, summary};

fn all(dir: &std::path::Path) -> Changes {
    let runtime = tokio::runtime::Runtime::new().expect("a runtime");
    runtime.block_on(async {
        let files = summary(dir).await.expect("a summary");
        changes(dir, &files, "HEAD").await
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

    let second = changes.head_of("src/lib.rs").expect("where it starts");
    assert_eq!(second, changes.files()[0].rows.len() + 1);
    assert_eq!(
        changes.at(second),
        Some(At::Band(&changes.files()[1])),
        "src/ opens a directory of its own"
    );
    assert_eq!(changes.at(second + 1), Some(At::Head(&changes.files()[1])));
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

#[test]
fn a_directory_is_named_once_over_the_files_that_share_it() {
    let files = ["src/a.rs", "src/b.rs", "docs/c.md", "top.rs"];
    let changes = Changes::new(
        files
            .iter()
            .map(|path| aligned(path, "one\n", "two\n"))
            .collect(),
    );
    let bands: Vec<&str> = (0..changes.rows())
        .filter_map(|row| match changes.at(row) {
            Some(At::Band(file)) => Some(file.dir()),
            _ => None,
        })
        .collect();
    assert_eq!(bands, ["src", "docs"], "one band a directory, in order");
    let heads: Vec<&str> = (0..changes.rows())
        .filter_map(|row| match changes.at(row) {
            Some(At::Head(file)) => Some(file.name()),
            _ => None,
        })
        .collect();
    assert_eq!(
        heads,
        ["a.rs", "b.rs", "c.md", "top.rs"],
        "every file, named"
    );
}

#[test]
fn a_folded_file_keeps_its_head_and_hides_its_rows() {
    let mut changes = Changes::new(vec![
        aligned("src/a.rs", "one\n", "ONE\n"),
        aligned("src/b.rs", "two\n", "TWO\n"),
    ]);
    let whole = changes.rows();
    let second = changes.head_of("src/b.rs").expect("the second file");

    changes.fold("src/a.rs");
    assert!(changes.is_folded("src/a.rs"));
    let file = changes.get("src/a.rs").expect("still there");
    assert_eq!(changes.shown(file), 0, "its rows are hidden");
    assert_eq!(
        changes.rows(),
        whole - file.rows.len(),
        "the surface is shorter"
    );
    assert_eq!(
        changes.head_of("src/b.rs"),
        Some(second - file.rows.len()),
        "what follows moves up"
    );
    assert!(matches!(changes.at(1), Some(At::Head(_))), "the head stays");
    assert!(
        matches!(changes.at(2), Some(At::Head(_))),
        "then the next file"
    );

    changes.fold("src/a.rs");
    assert!(!changes.is_folded("src/a.rs"));
    assert_eq!(changes.rows(), whole, "shown again");
}

#[test]
fn a_fold_outlives_the_file_being_aligned_again() {
    let mut changes = Changes::new(vec![aligned("src/a.rs", "one\n", "ONE\n")]);
    changes.fold("src/a.rs");
    changes.replace(aligned("src/a.rs", "one\n", "TWO\n"));
    assert!(
        changes.is_folded("src/a.rs"),
        "the keystroke did not open it"
    );
}

#[test]
fn a_fold_outlives_the_worktree_being_read_again() {
    let mut changes = Changes::new(vec![aligned("src/a.rs", "one\n", "ONE\n")]);
    changes.fold("src/a.rs");
    let mut again = Changes::new(vec![
        aligned("src/a.rs", "one\n", "ONE\n"),
        aligned("src/b.rs", "two\n", "TWO\n"),
    ]);
    again.refold(changes.folds());
    assert!(again.is_folded("src/a.rs"), "still shut");
    assert!(!again.is_folded("src/b.rs"), "a file it never shut");
}

#[test]
fn a_line_says_which_row_of_the_change_it_stands_on() {
    let change = Changes::new(vec![aligned("src/lib.rs", "one\ntwo\n", "one\nTWO\n")]);
    let row = change.row_of("src/lib.rs", 1).expect("the line's row");
    let At::Row(file, at) = change.at(row).expect("a row") else {
        panic!("the row of a line is a row");
    };
    assert_eq!(file.path, "src/lib.rs");
    assert_eq!(file.rows[at].new, Some(1));
}

#[test]
fn a_folded_file_stands_its_lines_on_no_row() {
    let mut change = Changes::new(vec![aligned("src/lib.rs", "one\ntwo\n", "one\nTWO\n")]);
    change.fold("src/lib.rs");
    assert_eq!(change.row_of("src/lib.rs", 1), None);
}

#[test]
fn a_line_no_file_of_the_change_has_stands_on_no_row() {
    let change = Changes::new(vec![aligned("src/lib.rs", "one\n", "two\n")]);
    assert_eq!(change.row_of("other.rs", 0), None);
    assert_eq!(change.row_of("src/lib.rs", 99), None);
}
