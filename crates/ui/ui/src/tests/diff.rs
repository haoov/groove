use groove_controllers::AppState;
use groove_controllers::workspace_service::from_text;
use groove_gfx::{CellSize, Fonts, Rect};

use crate::hit::Target;
use crate::input::{Delta, Input};
use crate::tests::{WINDOW, click, full_app, handle, window};
use crate::tokens::Tokens;
use crate::views::session::Tab;
use crate::widget::code_at;
use crate::{Ui, view};
use groove_types::{DiffView, LineMark};

const OLD: &str = "fn one() {}\nfn two() {}\nfn three() {}\n";
const NEW: &str = "fn one() {}\nfn TWO() {}\nfn three() {}\n";

fn opened() -> AppState {
    let mut app = with_files();
    app.workspace.opened = Some(from_text("src/lib.rs", OLD, NEW));
    app
}

fn with_files() -> AppState {
    let mut app = full_app();
    let worktree = app
        .session
        .selected()
        .and_then(|open| open.selected_worktree())
        .map(|w| w.id.clone())
        .expect("the fixture has a worktree");
    let file = groove_types::FileDiff {
        path: "src/lib.rs".into(),
        added: 1,
        deleted: 1,
        status: groove_types::FileStatus::Modified,
        staged: Some(false),
    };
    app.workspace.loaded(worktree, vec![file]);
    app
}

fn on_diff() -> Ui {
    let mut ui = Ui::default();
    ui.session.tab = Tab::Diff;
    ui
}

/// Every text the open file's rows drew.
fn row_texts(app: &AppState, ui: &Ui) -> Vec<String> {
    let (frame, hits) = view(app, ui, window(), &mut Fonts::embedded());
    let code = hits.rect_of(&Target::Code).expect("the rows are drawn");
    frame.layers()[0]
        .texts
        .iter()
        .filter(|run| code.contains(run.x, run.y))
        .map(|run| run.text.clone())
        .collect()
}

/// Every text the workspace drew.
fn texts(app: &AppState, ui: &Ui) -> Vec<String> {
    let (frame, _) = view(app, ui, window(), &mut Fonts::embedded());
    let workspace = crate::layout::Layout::of(window(), ui).workspace;
    frame.layers()[0]
        .texts
        .iter()
        .filter(|run| run.x >= workspace.x && run.x < workspace.right())
        .map(|run| run.text.clone())
        .collect()
}

#[test]
fn the_tab_draws_the_open_file_with_both_its_numbers() {
    let app = opened();
    let drawn = row_texts(&app, &on_diff());
    assert!(drawn.iter().any(|t| t == "two"), "the old side: {drawn:?}");
    assert!(drawn.iter().any(|t| t == "TWO"), "the new side");
    assert!(
        drawn.iter().filter(|t| *t == "fn").count() == 4,
        "a keyword per row, coloured on its own: {drawn:?}"
    );
    let numbers: Vec<&String> = drawn.iter().filter(|t| t.parse::<u32>().is_ok()).collect();
    assert_eq!(
        numbers,
        ["1", "2", "2", "3"],
        "one number a row, the old one only where a line went: {drawn:?}"
    );
}

#[test]
fn the_header_names_the_file_and_what_it_changed() {
    let app = opened();
    let drawn = texts(&app, &on_diff());
    assert!(drawn.iter().any(|t| t == "src/lib.rs"), "{drawn:?}");
    assert!(drawn.iter().any(|t| t == "+1"), "what it added: {drawn:?}");
    assert!(drawn.iter().any(|t| t == "-1"), "and what it took away");
}

#[test]
fn the_header_sits_under_the_tabs_and_rules_the_whole_width() {
    let app = opened();
    let ui = on_diff();
    let (frame, _) = view_of(&app, &ui);
    let tokens = Tokens::new(1.0);
    let workspace = crate::layout::Layout::of(window(), &ui).workspace;
    let band = Rect::new(
        workspace.x,
        workspace.y + tokens.row,
        workspace.w,
        tokens.row,
    );
    let styles = crate::style::Styles::new(app.config.theme(), tokens);
    let rule = frame.layers()[0]
        .quads
        .iter()
        .find(|quad| {
            quad.color == styles.line()
                && quad.rect.h == tokens.hairline
                && quad.rect.y == band.bottom() - tokens.hairline
        })
        .expect("a hairline under the header");
    assert_eq!(rule.rect.x, band.x, "from the left edge");
    assert_eq!(rule.rect.w, band.w, "to the right one");
    let path = frame.layers()[0]
        .texts
        .iter()
        .find(|run| run.text.ends_with("lib.rs"))
        .expect("the path");
    assert_eq!(path.y, band.y, "the band starts where the tabs end");
}

#[test]
fn a_long_path_keeps_its_end() {
    let long = "crates/ui/ui/src/views/session/components/diff.rs";
    let mut app = with_files();
    app.workspace.opened = Some(from_text(long, OLD, NEW));
    let drawn = texts(&app, &on_diff());
    let path = drawn
        .iter()
        .find(|text| text.ends_with("diff.rs"))
        .expect("the name survives: {drawn:?}");
    assert!(path.ends_with("components/diff.rs"), "{path}");
}

#[test]
fn a_row_says_what_it_is_with_its_ground_and_no_sign() {
    let app = opened();
    let ui = on_diff();
    let (frame, _) = view(&app, &ui, window(), &mut Fonts::embedded());
    let styles = crate::style::Styles::new(app.config.theme(), Tokens::new(1.0));
    let ground = |kind| {
        let color = styles.row_ground(kind, false).expect("a ground");
        frame.layers()[0]
            .quads
            .iter()
            .filter(|quad| quad.color == color)
            .count()
    };
    assert_eq!(ground(groove_types::RowKind::Added), 1);
    assert_eq!(ground(groove_types::RowKind::Removed), 1);
    let drawn = row_texts(&app, &ui);
    assert!(
        drawn
            .iter()
            .all(|t| !t.starts_with('+') && !t.starts_with('-')),
        "no signs: {drawn:?}"
    );
}

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
fn the_gutters_end_at_a_hairline_and_the_text_starts_past_it() {
    let app = opened();
    let ui = on_diff();
    let (frame, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let code = hits.rect_of(&Target::Code).expect("the rows are drawn");
    let styles = crate::style::Styles::new(app.config.theme(), Tokens::new(1.0));
    let rule = frame.layers()[0]
        .quads
        .iter()
        .find(|quad| quad.color == styles.line() && quad.rect.h == code.h && quad.rect.x > code.x)
        .expect("a hairline down the gutters");
    let numbers: Vec<f32> = frame.layers()[0]
        .texts
        .iter()
        .filter(|run| run.text.parse::<u32>().is_ok() && code.contains(run.x, run.y))
        .map(|run| run.x)
        .collect();
    assert!(!numbers.is_empty());
    assert!(
        numbers.iter().all(|x| *x < rule.rect.x),
        "the numbers sit left of it: {numbers:?} against {}",
        rule.rect.x
    );
    let text = frame.layers()[0]
        .texts
        .iter()
        .find(|run| run.text == "fn")
        .expect("a line of code");
    let gap = text.x - rule.rect.x;
    assert!(gap >= Tokens::new(1.0).md, "the text clears it: {gap}");
}

#[test]
fn a_gap_reads_as_a_band_across_the_rows() {
    let old: String = (1..=40).map(|n| format!("line {n}\n")).collect();
    let new = old
        .replace("line 1\n", "LINE 1\n")
        .replace("line 40\n", "LINE 40\n");
    let mut app = with_files();
    app.workspace.opened = Some(from_text("src/lib.rs", &old, &new));
    let ui = on_diff();
    let (frame, _) = view(&app, &ui, window(), &mut Fonts::embedded());
    let drawn = texts(&app, &ui);
    let band = drawn
        .iter()
        .find(|text| text.contains("lines"))
        .expect("the gap says how much it hides");
    assert!(band.starts_with('\u{2026}'), "{band}");
    let styles = crate::style::Styles::new(app.config.theme(), Tokens::new(1.0));
    let panels = frame.layers()[0]
        .quads
        .iter()
        .filter(|quad| quad.color == styles.panel() && quad.rect.h == Tokens::new(1.0).line)
        .count();
    assert_eq!(panels, 1, "one band, the height of a row");
}

/// The rows of one view, as the surface drew them, both panes in split.
fn in_view(app: &AppState, view: DiffView) -> Vec<String> {
    let mut ui = on_diff();
    ui.session.view = view;
    let (frame, _) = view_of(app, &ui);
    let workspace = crate::layout::Layout::of(window(), &ui).workspace;
    let body = workspace.y + Tokens::new(1.0).row * 2.0;
    frame.layers()[0]
        .texts
        .iter()
        .filter(|run| run.x >= workspace.x && run.x < workspace.right() && run.y >= body)
        .map(|run| run.text.clone())
        .collect()
}

fn view_of(app: &AppState, ui: &Ui) -> (groove_gfx::Frame, crate::Hits) {
    view(app, ui, window(), &mut Fonts::embedded())
}

#[test]
fn the_switch_names_the_three_views_and_picks_one() {
    let app = opened();
    let mut ui = on_diff();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    for view in DiffView::ALL {
        assert!(
            hits.rect_of(&Target::View(view)).is_some(),
            "{view:?} can be picked"
        );
    }
    let split = hits.rect_of(&Target::View(DiffView::Split)).expect("split");
    assert!(click(split, &mut ui, &app, &hits).is_empty());
    assert_eq!(ui.session.view, DiffView::Split);
}

/// The marks the file view drew, by colour.
fn marks(app: &AppState) -> Vec<LineMark> {
    let mut ui = on_diff();
    ui.session.view = DiffView::File;
    let (frame, _) = view_of(app, &ui);
    let styles = crate::style::Styles::new(app.config.theme(), Tokens::new(1.0));
    let width = Tokens::new(1.0).hairline * 2.0;
    frame.layers()[0]
        .quads
        .iter()
        .filter(|quad| quad.rect.w == width)
        .filter_map(|quad| {
            [LineMark::Added, LineMark::Removed, LineMark::Changed]
                .into_iter()
                .find(|mark| styles.mark(*mark) == quad.color)
        })
        .collect()
}

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

/// Thirty lines with one changed, far enough in for the alignment to elide the head.
fn long() -> AppState {
    many(30)
}

/// A file of `count` lines with one of them changed.
fn many(count: usize) -> AppState {
    let mut lines: Vec<String> = (1..=count).map(|at| "ab".repeat(at)).collect();
    let before = lines.join("\n") + "\n";
    lines[19] = "changed".into();
    let after = lines.join("\n") + "\n";
    let mut app = with_files();
    app.workspace.opened = Some(from_text("src/lib.rs", &before, &after));
    app
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
