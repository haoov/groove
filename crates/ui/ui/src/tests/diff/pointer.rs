//! What a click, the wheel and the keyboard do to the open file.

use super::*;
use crate::hit::Chars;
use groove_controllers::{Command, workspace};
use groove_types::{Caret, Edit, Motion};

#[test]
fn a_file_in_the_sidebar_opens_on_a_click() {
    let app = with_files();
    let mut ui = on_diff();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let rect = hits
        .rect_of(&Target::File("src/lib.rs".into()))
        .expect("the sidebar lists it");
    let commands = click(rect, &mut ui, &app, &hits);
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].id(), "workspace.open_file");
}

#[test]
fn the_wheel_over_the_workspace_scrolls_the_file() {
    let app = many(200);
    let mut ui = on_diff();
    ui.session.view = DiffView::File;
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let workspace = crate::layout::Layout::of(window(), &ui).workspace;
    let wheel = |pixels: f32, ui: &mut Ui| {
        handle(
            Input::Scroll {
                x: workspace.x + 1.0,
                y: workspace.y + 1.0,
                delta: Delta::Pixels(pixels),
            },
            ui,
            &app,
            &hits,
            window(),
        );
    };
    wheel(-40.0, &mut ui);
    assert_eq!(ui.session.diff, 40.0);
    assert_eq!(ui.session.files, 0.0, "the sidebar stayed where it was");

    wheel(-40_000.0, &mut ui);
    let bottom = ui.session.diff;
    assert!(bottom > 40.0, "it went down: {bottom}");
    wheel(40.0, &mut ui);
    assert_eq!(
        ui.session.diff,
        bottom - 40.0,
        "one notch back up moves at once"
    );
}

#[test]
fn a_click_in_the_file_lands_on_a_row_and_a_column() {
    let app = opened();
    let mut ui = on_diff();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let code = hits.rect_of(&Target::Code).expect("the rows are drawn");
    let tokens = Tokens::new(1.0);
    let point = (code.x + tokens.sm + 1.0, code.y + tokens.line * 2.0 + 1.0);
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
    let wanted = Edit::Move(Motion::To(Caret::new(1, 0)));
    assert_eq!(
        commands,
        [Command::Workspace(workspace::Command::Edit(wanted))],
        "the third row is the new side's second line"
    );
}

#[test]
fn a_click_on_a_removed_line_takes_no_caret() {
    let app = opened();
    let mut ui = on_diff();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let code = hits.rect_of(&Target::Code).expect("the rows are drawn");
    let tokens = Tokens::new(1.0);
    let commands = handle(
        Input::Press {
            x: code.x + tokens.sm + 1.0,
            y: code.y + tokens.line + 1.0,
        },
        &mut ui,
        &app,
        &hits,
        window(),
    );
    assert!(commands.is_empty(), "the second row is the line that went");
}

#[test]
fn a_point_outside_the_rows_lands_nowhere() {
    let tokens = Tokens::new(1.0);
    let chars = Chars {
        left: 40.0,
        advance: 8.0,
        scroll: 0.0,
    };
    let rect = Rect::new(0.0, 0.0, 100.0, 100.0);
    assert!(code_at(&tokens, chars, rect, (-1.0, 10.0)).is_none());
    let scrolled = Chars {
        scroll: tokens.line,
        ..chars
    };
    assert_eq!(
        code_at(&tokens, scrolled, rect, (chars.left, 0.0)),
        Some((1, 0)),
        "a scrolled surface counts from the first row"
    );
    let _ = WINDOW;
}

#[test]
fn a_click_lands_on_the_character_it_points_at() {
    let app = opened();
    let ui = on_diff();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let rect = hits.rect_of(&Target::Code).expect("the rows are drawn");
    let chars = hits.chars();
    assert!(
        chars.left > rect.x + Tokens::new(1.0).sm,
        "the text starts past the gutter, not at the surface edge"
    );
    let tokens = Tokens::new(1.0);
    let at = |x: f32| code_at(&tokens, chars, rect, (x, rect.y + 1.0)).map(|at| at.1);
    assert_eq!(at(chars.left + 1.0), Some(0), "the first character");
    assert_eq!(at(chars.left - 4.0), Some(0), "the gutter is column zero");
    for column in [1, 5, 9] {
        let middle = chars.left + chars.advance * column as f32 + chars.advance / 4.0;
        assert_eq!(at(middle), Some(column), "column {column}");
    }
    let past = chars.left + chars.advance * 3.0 + chars.advance * 0.8;
    assert_eq!(
        at(past),
        Some(4),
        "past the middle of a character is after it"
    );
}

#[test]
fn the_caret_shows_only_where_the_keyboard_is() {
    let app = opened();
    let mut ui = on_diff();
    ui.focus = crate::Focus::Workspace;
    let styles = crate::style::Styles::new(app.config.theme(), Tokens::new(1.0));
    let rules = |ui: &Ui| {
        let (frame, hits) = view_of(&app, ui);
        let code = hits.rect_of(&Target::Code).expect("the rows are drawn");
        frame.layers()[0]
            .quads
            .iter()
            .filter(|quad| quad.color == styles.here() && quad.rect.h == Tokens::new(1.0).hairline)
            .filter(|quad| code.contains(quad.rect.x + 1.0, quad.rect.y))
            .count()
    };
    assert_eq!(rules(&ui), 2, "a rule above the caret's row and one below");
    ui.focus = crate::Focus::Agent;
    assert_eq!(rules(&ui), 0, "and none once the keyboard leaves");
}

#[test]
fn what_is_held_reads_apart_from_the_row_the_caret_is_on() {
    let app = opened();
    let ui = on_diff();
    let styles = crate::style::Styles::new(app.config.theme(), Tokens::new(1.0));
    let tokens = Tokens::new(1.0);
    assert_ne!(
        styles.held(),
        styles.here(),
        "a selection and a caret row are told apart by colour"
    );
    let (frame, _) = view_of(&app, &ui);
    let rows = frame.layers()[0]
        .quads
        .iter()
        .filter(|quad| quad.rect.h == tokens.line && quad.color == styles.here())
        .count();
    assert_eq!(rows, 0, "the caret's row carries no ground of its own");
}

#[test]
fn the_sidebar_says_which_file_is_open() {
    let app = opened();
    let ui = on_diff();
    let (frame, hits) = view_of(&app, &ui);
    let row = hits
        .rect_of(&Target::File("src/lib.rs".into()))
        .expect("the file's row");
    let styles = crate::style::Styles::new(app.config.theme(), Tokens::new(1.0));
    let hairline = Tokens::new(1.0).hairline;
    let rules = frame.layers()[0]
        .quads
        .iter()
        .filter(|quad| quad.color == styles.here() && quad.rect.h == hairline)
        .filter(|quad| quad.rect.x == row.x && quad.rect.w == row.w)
        .count();
    assert_eq!(rules, 2, "the open file's row is ruled above and below");
}

#[test]
fn changing_the_view_keeps_the_same_line_in_view() {
    let app = long();
    let mut ui = on_diff();
    ui.session.view = DiffView::Inline;
    let height = Tokens::new(1.0).line;
    ui.session.diff = height * 6.0;
    let (_, hits) = view_of(&app, &ui);
    let file = hits.rect_of(&Target::View(DiffView::File)).expect("file");
    assert!(click(file, &mut ui, &app, &hits).is_empty());
    assert_eq!(
        ui.session.diff,
        height * 18.0,
        "row 6 of the diff is line 19, the file view's row 18"
    );

    let inline = hits.rect_of(&Target::View(DiffView::Inline)).expect("back");
    assert!(click(inline, &mut ui, &app, &hits).is_empty());
    assert_eq!(
        ui.session.diff,
        height * 6.0,
        "and back to the row it came from"
    );
}

#[test]
fn a_wheel_notch_over_the_file_moves_one_code_line() {
    let app = many(200);
    let mut ui = on_diff();
    ui.session.view = DiffView::File;
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let workspace = crate::layout::Layout::of(window(), &ui).workspace;
    handle(
        Input::Scroll {
            x: workspace.x + 1.0,
            y: workspace.y + 1.0,
            delta: Delta::Lines(-1.0),
        },
        &mut ui,
        &app,
        &hits,
        window(),
    );
    let tokens = Tokens::new(1.0);
    assert_eq!(
        ui.session.diff, tokens.line,
        "a code row, not a list row of {}",
        tokens.row
    );
}

#[test]
fn a_click_lands_when_the_scroll_sits_past_what_the_file_has() {
    let app = opened();
    let mut ui = on_diff();
    ui.session.diff = 10_000.0;
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let code = hits.rect_of(&Target::Code).expect("the rows are drawn");
    let tokens = Tokens::new(1.0);
    let point = (code.x + tokens.sm + 1.0, code.y + tokens.line * 2.0 + 1.0);
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
    let wanted = Edit::Move(Motion::To(Caret::new(1, 0)));
    assert_eq!(
        commands,
        [Command::Workspace(workspace::Command::Edit(wanted))],
        "the click reads the rows the frame drew, not a scroll it clamped away"
    );
}
