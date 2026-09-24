use groove_text::Document;

use groove_types::{Row, RowKind};

use crate::align;
use crate::alignment::CONTEXT;

fn doc(text: &str) -> Document {
    Document::new("a.rs", text)
}

/// Every row as `(old, new, kind)`, with lines counted from one as a reader sees them.
fn rows(old: &str, new: &str, context: u32) -> Vec<(Option<u32>, Option<u32>, RowKind)> {
    align(&doc(old), &doc(new), context, &[])
        .into_iter()
        .map(|row: Row| (row.old.map(|at| at + 1), row.new.map(|at| at + 1), row.kind))
        .collect()
}

#[test]
fn a_file_that_did_not_change_is_all_context() {
    let text = "one\ntwo\nthree\n";
    assert_eq!(
        rows(text, text, CONTEXT),
        [
            (Some(1), Some(1), RowKind::Context),
            (Some(2), Some(2), RowKind::Context),
            (Some(3), Some(3), RowKind::Context),
        ]
    );
}

#[test]
fn a_changed_line_reads_as_a_removal_then_an_addition() {
    let rows = rows("one\ntwo\nthree\n", "one\nTWO\nthree\n", CONTEXT);
    assert_eq!(
        rows,
        [
            (Some(1), Some(1), RowKind::Context),
            (Some(2), None, RowKind::Removed),
            (None, Some(2), RowKind::Added),
            (Some(3), Some(3), RowKind::Context),
        ]
    );
}

#[test]
fn an_addition_and_a_removal_keep_their_own_numbers() {
    let added = rows("one\ntwo\n", "one\nadded\ntwo\n", CONTEXT);
    assert_eq!(
        added,
        [
            (Some(1), Some(1), RowKind::Context),
            (None, Some(2), RowKind::Added),
            (Some(2), Some(3), RowKind::Context),
        ]
    );

    let removed = rows("one\ngone\ntwo\n", "one\ntwo\n", CONTEXT);
    assert_eq!(
        removed,
        [
            (Some(1), Some(1), RowKind::Context),
            (Some(2), None, RowKind::Removed),
            (Some(3), Some(2), RowKind::Context),
        ]
    );
}

#[test]
fn the_unchanged_middle_of_a_long_file_becomes_a_gap() {
    let old: String = (1..=30).map(|n| format!("line {n}\n")).collect();
    let new = old
        .replace("line 1\n", "LINE 1\n")
        .replace("line 30\n", "LINE 30\n");
    let rows = rows(&old, &new, CONTEXT);
    let gaps: Vec<RowKind> = rows
        .iter()
        .map(|(_, _, kind)| *kind)
        .filter(|kind| matches!(kind, RowKind::Gap(_)))
        .collect();
    assert_eq!(gaps, [RowKind::Gap(22)], "{rows:?}");
    assert_eq!(
        rows.len(),
        2 + 3 + 1 + 3 + 2,
        "two changes, their context, one gap"
    );
}

#[test]
fn a_new_file_is_all_additions_and_an_emptied_one_all_removals() {
    let fresh = rows("", "one\ntwo\n", CONTEXT);
    assert_eq!(
        fresh,
        [
            (None, Some(1), RowKind::Added),
            (None, Some(2), RowKind::Added),
        ]
    );
    let emptied = rows("one\ntwo\n", "", CONTEXT);
    assert_eq!(
        emptied,
        [
            (Some(1), None, RowKind::Removed),
            (Some(2), None, RowKind::Removed),
        ]
    );
}

#[test]
fn a_moved_block_reads_as_a_removal_and_an_addition_not_a_rewrite() {
    let old = "a\nb\nc\nd\ne\nf\ng\nh\n";
    let new = "c\nd\ne\nf\ng\nh\na\nb\n";
    let rows = rows(old, new, CONTEXT);
    let changed = rows
        .iter()
        .filter(|(_, _, kind)| matches!(kind, RowKind::Removed | RowKind::Added))
        .count();
    assert_eq!(
        changed, 4,
        "two lines each way, not the whole file: {rows:?}"
    );
}
