//! What a click, the wheel and the keyboard do to the open file.

use super::*;

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
    handle(
        Input::Press {
            x: point.0,
            y: point.1,
        },
        &mut ui,
        &app,
        &hits,
        window(),
    );
    assert_eq!(ui.session.at, Some((2, 0)), "the third row, first column");
}

#[test]
fn a_point_outside_the_rows_lands_nowhere() {
    let tokens = Tokens::new(1.0);
    let cell = CellSize {
        width: 8.0,
        height: 17.0,
    };
    let rect = Rect::new(0.0, 0.0, 100.0, 100.0);
    assert!(code_at(&tokens, cell, rect, 0.0, (-1.0, 10.0)).is_none());
    assert_eq!(
        code_at(&tokens, cell, rect, tokens.line, (tokens.sm, 0.0)),
        Some((1, 0)),
        "a scrolled surface counts from the first row"
    );
    let _ = WINDOW;
}

#[test]
fn the_caret_shows_only_where_the_keyboard_is() {
    let app = opened();
    let mut ui = on_diff();
    ui.session.at = Some((0, 0));
    ui.focus = crate::Focus::Workspace;
    let styles = crate::style::Styles::new(app.config.theme(), Tokens::new(1.0));
    let raised = |ui: &Ui| {
        let (frame, _) = view_of(&app, ui);
        frame.layers()[0]
            .quads
            .iter()
            .filter(|quad| quad.color == styles.raised() && quad.rect.h == Tokens::new(1.0).line)
            .count()
    };
    assert_eq!(raised(&ui), 1, "the row the caret is on");
    ui.focus = crate::Focus::Agent;
    assert_eq!(raised(&ui), 0, "and nothing once the keyboard leaves");
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
    assert!(
        frame.layers()[0]
            .quads
            .iter()
            .any(|quad| quad.color == styles.raised() && quad.rect == row),
        "the open file's row is raised"
    );
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
