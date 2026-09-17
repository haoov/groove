//! What each of the three views shows.

use super::*;
use groove_controllers::{Command, workspace};
use groove_types::{Caret, Edit, Motion};

use crate::hit::Target;
use crate::input::{Input, handle};

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

/// A Go file, whose own formatter writes tabs.
fn tabbed() -> AppState {
    let before = "func one() {\n\treturn 1\n}\n";
    let after = "func one() {\n\treturn 2\n}\n";
    let mut app = with_files();
    app.workspace.opened = Some(from_text("main.go", before, after));
    app
}

#[test]
fn a_tab_is_drawn_run_out_to_its_stop() {
    let drawn = in_view(&tabbed(), DiffView::File);
    let joined = drawn.join("");
    assert!(
        joined.contains("    return"),
        "the tab reads as four columns: {drawn:?}"
    );
    assert!(
        !joined.contains('\t'),
        "and no tab reaches the glyphs, which would set its own stop"
    );
}

#[test]
fn a_click_past_a_tab_lands_on_the_character_it_points_at() {
    let app = tabbed();
    let mut ui = on_diff();
    ui.session.view = DiffView::File;
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let code = hits.rect_of(&Target::Code).expect("the rows are drawn");
    let chars = hits.chars();
    let tokens = Tokens::new(1.0);
    let point = (
        chars.left + chars.advance * 4.0 + 1.0,
        code.y + tokens.line + 1.0,
    );
    let commands = handle(
        Input::Press {
            x: point.0,
            y: point.1,
        },
        &mut ui,
        &app,
        &hits,
        window(),
    );
    let wanted = Edit::Move(Motion::To(Caret::new(1, 1)));
    assert_eq!(
        commands,
        [Command::Workspace(workspace::Command::Edit(wanted))],
        "the fifth column is the character after the tab, not the fifth character"
    );
}
