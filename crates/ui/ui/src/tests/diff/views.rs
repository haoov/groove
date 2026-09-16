//! What each of the three views shows.

use super::*;

#[test]
fn the_file_view_marks_a_changed_line_and_grounds_nothing() {
    let app = opened();
    assert_eq!(
        marks(&app),
        [LineMark::Changed],
        "one line went and one came in its place"
    );
    let mut ui = on_diff();
    ui.session.view = DiffView::File;
    let (frame, _) = view_of(&app, &ui);
    let styles = crate::style::Styles::new(app.config.theme(), Tokens::new(1.0));
    let grounds = frame.layers()[0]
        .quads
        .iter()
        .filter(|quad| {
            [groove_types::RowKind::Added, groove_types::RowKind::Removed]
                .into_iter()
                .filter_map(|kind| styles.row_ground(kind, false))
                .any(|color| color == quad.color)
        })
        .count();
    assert_eq!(grounds, 0, "the file view tints no row");
}

#[test]
fn a_line_added_and_a_line_removed_are_marked_apart() {
    let mut app = with_files();
    app.workspace.opened = Some(from_text(
        "src/lib.rs",
        "one\ngone\nthree\n",
        "one\nthree\nadded\n",
    ));
    assert_eq!(
        marks(&app),
        [LineMark::Removed, LineMark::Added],
        "the deletion marks the line that closed it, the addition its own"
    );
}

#[test]
fn the_file_view_shows_what_is_there_now_and_nothing_that_went() {
    let drawn = in_view(&opened(), DiffView::File);
    assert!(drawn.iter().any(|t| t == "TWO"), "the new line: {drawn:?}");
    assert!(!drawn.iter().any(|t| t == "two"), "not the old one");
    let numbers: Vec<&String> = drawn.iter().filter(|t| t.parse::<u32>().is_ok()).collect();
    assert_eq!(numbers, ["1", "2", "3"], "the file's own numbers");
}

#[test]
fn split_draws_each_side_with_its_own_numbers() {
    let app = opened();
    let drawn = in_view(&app, DiffView::Split);
    assert!(drawn.iter().any(|t| t == "two"), "the old side: {drawn:?}");
    assert!(drawn.iter().any(|t| t == "TWO"), "and the new one");
    let numbers: Vec<&String> = drawn.iter().filter(|t| t.parse::<u32>().is_ok()).collect();
    assert_eq!(
        numbers,
        ["1", "2", "3", "1", "2", "3"],
        "three rows a side, each numbered once: {drawn:?}"
    );
}

#[test]
fn split_puts_a_rule_between_the_sides() {
    let app = opened();
    let mut ui = on_diff();
    ui.session.view = DiffView::Split;
    let (frame, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let code = hits
        .rect_of(&Target::Code)
        .expect("the new side takes clicks");
    let styles = crate::style::Styles::new(app.config.theme(), Tokens::new(1.0));
    let rules = frame.layers()[0]
        .quads
        .iter()
        .filter(|quad| quad.color == styles.line() && quad.rect.h == code.h)
        .count();
    assert_eq!(rules, 3, "one between the sides, one per gutter");
    assert!(code.x > code.w, "clicks land on the right half");
}

#[test]
fn the_file_view_draws_the_whole_file_not_the_alignment() {
    let inline = in_view(&long(), DiffView::Inline);
    assert!(
        inline.iter().any(|t| t.starts_with('\u{2026}')),
        "the diff elides the head: {inline:?}"
    );
    let file = in_view(&long(), DiffView::File);
    assert!(
        !file.iter().any(|t| t.starts_with('\u{2026}')),
        "the file view elides nothing"
    );
    let numbers: Vec<&String> = file.iter().filter(|t| t.parse::<u32>().is_ok()).collect();
    assert_eq!(numbers[..3], ["1", "2", "3"], "from the first line");
    assert_eq!(numbers.len(), 30, "every line: {numbers:?}");
}
