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

#[test]
fn every_file_is_named_by_its_own_head_row() {
    let files = ["src/a.rs", "src/b.rs", "docs/c.md", "top.rs"];
    let changes = Changes::new(
        files
            .iter()
            .map(|path| aligned(path, "one\n", "two\n"))
            .collect(),
    );
    let heads: Vec<&str> = (0..changes.rows())
        .filter_map(|row| match changes.at(row) {
            Some(At::Head(file)) => Some(file.path.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        heads,
        ["src/a.rs", "src/b.rs", "docs/c.md", "top.rs"],
        "every file, named by its path"
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
    assert!(matches!(changes.at(0), Some(At::Head(_))), "the head stays");
    assert!(
        matches!(changes.at(1), Some(At::Head(_))),
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

/// A file that changes at both ends, so one gap stands between them.
fn far_apart() -> (String, String) {
    let before: String = (0..40).map(|at| format!("line {at}\n")).collect();
    let after = before
        .replace("line 0\n", "LINE 0\n")
        .replace("line 39\n", "LINE 39\n");
    (before, after)
}

#[test]
#[allow(clippy::single_range_in_vec_init)]
fn a_gap_gives_up_the_lines_the_reader_opens() {
    let (before, after) = far_apart();
    let shut = crate::changes::aligned("src/lib.rs", &before, &after);
    let gaps: Vec<u32> = shut
        .rows
        .iter()
        .filter_map(|row| match row.kind {
            RowKind::Gap(lines) => Some(lines),
            _ => None,
        })
        .collect();
    assert_eq!(gaps, [32], "one gap, of everything between the two changes");

    let sides = (
        groove_text::Document::plain("src/lib.rs", &before),
        groove_text::Document::plain("src/lib.rs", &after),
    );
    let open = crate::changes::from_sides("src/lib.rs", &sides.0, &sides.1, &[4..14]);
    let gaps: Vec<u32> = open
        .rows
        .iter()
        .filter_map(|row| match row.kind {
            RowKind::Gap(lines) => Some(lines),
            _ => None,
        })
        .collect();
    assert_eq!(gaps, [22], "the rest of it stays hidden");
    assert!(
        open.lines.iter().any(|line| line == "line 4"),
        "the first line it gave up: {:?}",
        open.lines
    );
    assert!(
        open.lines.iter().any(|line| line == "line 13"),
        "and the last one"
    );
    assert!(
        !open.lines.iter().any(|line| line == "line 14"),
        "and no more than that"
    );
}

#[test]
#[allow(clippy::single_range_in_vec_init)]
fn opening_a_gap_twice_joins_what_it_gave_up() {
    let (before, after) = far_apart();
    let (old, new) = (
        groove_text::Document::plain("src/lib.rs", &before),
        groove_text::Document::plain("src/lib.rs", &after),
    );
    let mut changes = Changes::new(vec![aligned("src/lib.rs", &before, &after)]);
    let whole = changes.rows();

    changes.open_gap("src/lib.rs", 4..14, &old, &new);
    assert_eq!(changes.opened_of("src/lib.rs"), &[4..14]);
    assert_eq!(changes.rows(), whole + 10, "ten more rows stand");

    changes.open_gap("src/lib.rs", 10..20, &old, &new);
    assert_eq!(
        changes.opened_of("src/lib.rs"),
        &[4..20],
        "the two runs stand as one"
    );
    assert_eq!(changes.rows(), whole + 16);
}
