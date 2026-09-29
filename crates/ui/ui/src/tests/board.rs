//! The board: what its columns hold, and how the window gets to it and back.

mod attention;
mod filter;
mod plan;
mod review;
mod timeline;

use groove_controllers::AppState;
use groove_gfx::Fonts;
use groove_types::{ExternalId, SessionKind};

use crate::hit::{Hits, Target};
use crate::input::Key;
use crate::tests::{ALT, click, full_app, press, task, window};
use crate::{Surface, Ui, view};

fn on_board(app: &AppState) -> (Ui, Hits) {
    let ui = Ui {
        surface: Surface::Board,
        ..Ui::default()
    };
    let (_, hits) = view(app, &ui, window(), &mut Fonts::embedded());
    (ui, hits)
}

/// Every text the board drew.
fn texts(app: &AppState, ui: &Ui) -> Vec<String> {
    let (frame, _) = view(app, ui, window(), &mut Fonts::embedded());
    frame.layers()[0]
        .texts
        .iter()
        .map(|run| run.text.clone())
        .collect()
}

#[test]
fn the_rail_s_board_row_opens_the_board_and_reads_the_sources() {
    let app = full_app();
    let mut ui = Ui::default();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let row = hits
        .rect_of(&Target::Board)
        .expect("the rail names the board");
    let commands = click(row, &mut ui, &app, &hits);
    assert_eq!(ui.surface, Surface::Board);
    assert_eq!(
        commands.iter().map(|c| c.id()).collect::<Vec<_>>(),
        ["session.list", "task.load", "delivery.review_queue"]
    );
}

#[test]
fn the_key_into_the_board_reads_what_a_click_reads() {
    let app = full_app();
    let mut ui = Ui::default();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let row = hits.rect_of(&Target::Board).expect("the board row");
    let clicked = click(row, &mut ui, &app, &hits);
    ui.surface = Surface::Session;
    let pressed = crate::tests::press(crate::input::Key::Char('h'), ALT, &mut ui, &app);
    assert_eq!(
        pressed.iter().map(|c| c.id()).collect::<Vec<_>>(),
        clicked.iter().map(|c| c.id()).collect::<Vec<_>>()
    );
}

#[test]
fn the_board_lists_the_open_sessions_and_the_tasks_with_none() {
    let mut app = full_app();
    app.task.tasks = vec![
        task("gh-a-b-1", "waiting one", "github.com/a/b#1"),
        task("gh-a-b-2", "waiting two", "github.com/a/b#2"),
    ];
    let (ui, _) = on_board(&app);
    let drawn = texts(&app, &ui);
    assert!(drawn.iter().any(|t| t == "LIVE · 2"), "{drawn:?}");
    assert!(drawn.iter().any(|t| t == "UP NEXT · 2"), "{drawn:?}");
    assert!(drawn.iter().any(|t| t == "waiting one"), "{drawn:?}");
    assert!(drawn.iter().any(|t| t.contains("high")), "{drawn:?}");
}

#[test]
fn a_task_with_a_session_of_its_own_is_live_and_not_up_next() {
    let mut app = full_app();
    let external = ExternalId::new("github.com/a/b#1");
    app.task.tasks = vec![task("gh-a-b-1", "already open", external.as_str())];
    let kind = SessionKind::Task {
        external_id: external,
    };
    app.session
        .open
        .first_mut()
        .expect("the fixture has one")
        .session
        .kind = kind.clone();
    app.session
        .living
        .first_mut()
        .expect("and it lives")
        .session
        .kind = kind;
    let (ui, _) = on_board(&app);
    let drawn = texts(&app, &ui);
    assert!(drawn.iter().any(|t| t == "UP NEXT"), "no count: {drawn:?}");
    assert!(!drawn.iter().any(|t| t == "already open"), "{drawn:?}");
}

#[test]
fn the_chord_opens_the_board_and_closes_it_again() {
    let app = full_app();
    let mut ui = Ui::default();
    let opened = press(Key::Char('h'), ALT, &mut ui, &app);
    assert_eq!(ui.surface, Surface::Board);
    assert_eq!(
        opened.iter().map(|c| c.id()).collect::<Vec<_>>(),
        ["session.list", "task.load", "delivery.review_queue"],
        "the board reads all three of its columns"
    );
    let closed = press(Key::Char('h'), ALT, &mut ui, &app);
    assert_eq!(ui.surface, Surface::Session);
    assert!(closed.is_empty(), "nothing is read on the way out");
}

#[test]
fn the_rail_holds_no_selection_while_the_board_is_up() {
    let app = full_app();
    let ruled = |surface| {
        let ui = Ui {
            surface,
            ..Ui::default()
        };
        let (frame, _) = view(&app, &ui, window(), &mut Fonts::embedded());
        let styles = groove_ui_kit::base::style::Styles::new(
            app.config.theme(),
            groove_ui_kit::base::tokens::Tokens::new(1.0),
        );
        let rail = crate::layout::Layout::of(window(), &ui).rail;
        frame.layers()[0]
            .quads
            .iter()
            .filter(|quad| quad.color == styles.chosen() && quad.rect.x < rail.right())
            .count()
    };
    assert_eq!(
        ruled(Surface::Session),
        2,
        "the selected session's two rules"
    );
    assert_eq!(
        ruled(Surface::Board),
        2,
        "the board's own row, and no session"
    );
}

#[test]
fn a_session_picked_while_the_board_is_up_puts_the_window_back_on_it() {
    let app = full_app();
    let (mut ui, hits) = on_board(&app);
    let id = app
        .session
        .open
        .first()
        .map(|open| open.session.id.clone())
        .expect("the fixture has a session");
    let row = hits.rect_of(&Target::Session(id)).expect("a row names it");
    let commands = click(row, &mut ui, &app, &hits);
    assert_eq!(ui.surface, Surface::Session);
    assert_eq!(
        commands.iter().map(|c| c.id()).collect::<Vec<_>>(),
        ["session.open"]
    );
}

#[test]
fn every_row_of_a_column_ends_in_a_hairline() {
    let mut app = full_app();
    app.task.tasks = vec![
        task("gh-a-b-1", "waiting one", "github.com/a/b#1"),
        task("gh-a-b-2", "waiting two", "github.com/a/b#2"),
        task("gh-a-b-3", "waiting three", "github.com/a/b#3"),
    ];
    let ui = Ui {
        surface: Surface::Board,
        ..Ui::default()
    };
    let (frame, _) = view(&app, &ui, window(), &mut Fonts::embedded());
    let tokens = groove_ui_kit::base::tokens::Tokens::new(1.0);
    let styles = groove_ui_kit::base::style::Styles::new(app.config.theme(), tokens);
    let board = crate::layout::Layout::of(window(), &ui).board;
    let width = (board.w / 3.0).floor();
    let rules = frame.layers()[0]
        .quads
        .iter()
        .filter(|quad| quad.color == styles.line() && quad.rect.h == tokens.hairline)
        .filter(|quad| quad.rect.x >= board.x + width && quad.rect.x < board.x + width * 2.0)
        .count();
    assert_eq!(
        rules, 5,
        "the heading, the three rows, and the divider's own"
    );
}

#[test]
fn closing_the_last_session_leaves_the_board_showing() {
    let mut app = full_app();
    let ui = Ui::default();
    assert_eq!(ui.showing(&app), Surface::Session, "a session is open");
    app.session.open.clear();
    app.session.selected = None;
    assert_eq!(ui.showing(&app), Surface::Board, "nothing is left to show");
}
