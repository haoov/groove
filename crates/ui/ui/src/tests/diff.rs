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
    let app = opened();
    let mut ui = on_diff();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let workspace = crate::layout::Layout::of(window(), &ui).workspace;
    handle(
        Input::Scroll {
            x: workspace.x + 1.0,
            y: workspace.y + 1.0,
            delta: Delta::Pixels(-40.0),
        },
        &mut ui,
        &app,
        &hits,
        window(),
    );
    assert_eq!(ui.session.diff, 40.0);
    assert_eq!(ui.session.files, 0.0, "the sidebar stayed where it was");
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
