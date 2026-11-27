use groove_types::RowKind;

use crate::tests::summary::{repo, write};
use crate::{Changes, aligned, changes, summary};

fn all(dir: &std::path::Path) -> Changes {
    let runtime = tokio::runtime::Runtime::new().expect("a runtime");
    runtime.block_on(async {
        let files = summary(dir).await.expect("a summary");
        changes(dir, &files, "HEAD").await.expect("the changes")
    })
}

#[test]
fn a_file_gives_every_row_its_text() {
    let file = aligned("src/lib.rs", "one\ntwo\n", "one\nTWO\n");
    let kinds: Vec<RowKind> = file.rows().map(|row| row.kind).collect();
    assert_eq!(
        kinds,
        [RowKind::Context, RowKind::Removed, RowKind::Added],
        "{file:?}"
    );
    assert_eq!(file.texts().collect::<Vec<_>>(), ["one", "two", "TWO"]);
    assert_eq!(file.indent, 4, "rust indents four");
}

#[test]
fn a_gap_says_how_many_lines_it_hides() {
    let before: String = (0..40).map(|at| format!("line {at}\n")).collect();
    let after = before.replace("line 0\n", "first\n");
    let file = aligned("src/lib.rs", &before, &after);
    let gap = file
        .rows()
        .position(|row| matches!(row.kind, RowKind::Gap(_)))
        .expect("a gap");
    assert_eq!(file.text(gap), "\u{2026} 36 lines");
}

#[test]
fn every_file_of_the_worktree_stands_in_the_change() {
    let dir = repo();
    write(dir.path(), "src/lib.rs", "fn one() {}\nfn TWO() {}\n");
    write(dir.path(), "README.md", "# title\n## added\n");
    let changes = all(dir.path());
    assert_eq!(changes.files().len(), 2, "{:?}", changes.files());
    assert_eq!(changes.rows(), 0, "no file focused, no row");
}

#[test]
fn a_file_with_no_head_side_is_all_additions() {
    let dir = repo();
    write(dir.path(), "src/new.rs", "fn new() {}\n");
    let changes = all(dir.path());
    let new = changes.get("src/new.rs").expect("the untracked file");
    assert_eq!(new.len(), 1);
    assert_eq!(new.row(0).unwrap().kind, RowKind::Added);
    assert_eq!(new.texts().collect::<Vec<_>>(), ["fn new() {}"]);
}

#[test]
fn a_line_says_which_row_of_the_change_it_stands_on() {
    let mut change = Changes::new(vec![aligned("src/lib.rs", "one\ntwo\n", "one\nTWO\n")]);
    change.focus(Some("src/lib.rs".into()));
    let row = change.row_of("src/lib.rs", 1).expect("the line's row");
    let (file, at) = change.at(row).expect("a row");
    assert_eq!(file.path, "src/lib.rs");
    assert_eq!(file.row(at).unwrap().new, Some(1));
}

#[test]
fn a_line_no_file_of_the_change_has_stands_on_no_row() {
    let mut change = Changes::new(vec![aligned("src/lib.rs", "one\n", "two\n")]);
    change.focus(Some("src/lib.rs".into()));
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
        .rows()
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
        .rows()
        .filter_map(|row| match row.kind {
            RowKind::Gap(lines) => Some(lines),
            _ => None,
        })
        .collect();
    assert_eq!(gaps, [22], "the rest of it stays hidden");
    assert!(
        open.texts().any(|line| line == "line 4"),
        "the first line it gave up: {:?}",
        open.texts().collect::<Vec<_>>()
    );
    assert!(
        open.texts().any(|line| line == "line 13"),
        "and the last one"
    );
    assert!(
        !open.texts().any(|line| line == "line 14"),
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
    changes.focus(Some("src/lib.rs".into()));
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
