//! The board: what its columns hold, and how the window gets to it and back.

use groove_controllers::AppState;
use groove_gfx::Fonts;
use groove_types::{ExternalId, SessionKind};

use crate::hit::{Hits, Target};
use crate::input::{Key, Modifiers};
use crate::tests::{CHORD, click, full_app, press, task, window};
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
        ["session.list", "task.load"]
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
    let opened = press(Key::Char('k'), CHORD, &mut ui, &app);
    assert_eq!(ui.surface, Surface::Board);
    assert_eq!(
        opened.iter().map(|c| c.id()).collect::<Vec<_>>(),
        ["session.list", "task.load"]
    );
    let closed = press(Key::Char('k'), CHORD, &mut ui, &app);
    assert_eq!(ui.surface, Surface::Session);
    assert!(closed.is_empty(), "nothing is read on the way out");
}

#[test]
fn the_rail_holds_no_selection_while_the_board_is_up() {
    let app = full_app();
    let raised = |surface| {
        let ui = Ui {
            surface,
            ..Ui::default()
        };
        let (frame, _) = view(&app, &ui, window(), &mut Fonts::embedded());
        let styles = crate::style::Styles::new(app.config.theme(), crate::tokens::Tokens::new(1.0));
        let rail = crate::layout::Layout::of(window(), &ui).rail;
        frame.layers()[0]
            .quads
            .iter()
            .filter(|quad| quad.color == styles.raised() && quad.rect.x < rail.right())
            .count()
    };
    assert_eq!(raised(Surface::Session), 1, "the selected session");
    assert_eq!(
        raised(Surface::Board),
        1,
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
    let tokens = crate::tokens::Tokens::new(1.0);
    let styles = crate::style::Styles::new(app.config.theme(), tokens);
    let board = crate::layout::Layout::of(window(), &ui).board;
    let width = (board.w / 3.0).floor();
    let rules = frame.layers()[0]
        .quads
        .iter()
        .filter(|quad| quad.color == styles.line() && quad.rect.h == tokens.hairline)
        .filter(|quad| quad.rect.x >= board.x + width && quad.rect.x < board.x + width * 2.0)
        .count();
    assert_eq!(
        rules, 4,
        "one under the heading and one under each of the three"
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

#[test]
fn a_live_item_opens_to_show_its_worktrees() {
    let app = full_app();
    let (mut ui, hits) = on_board(&app);
    let id = app
        .session
        .open
        .first()
        .map(|open| open.session.id.clone())
        .expect("the fixture has a session");
    let before = texts(&app, &ui);
    assert!(!before.iter().any(|t| t == "explorer/alpha"), "{before:?}");

    let twisty = hits
        .rect_of(&Target::Unfold(id.clone()))
        .expect("the item has a twisty");
    assert!(click(twisty, &mut ui, &app, &hits).is_empty(), "no command");
    assert!(ui.board.is_open(&id));
    let after = texts(&app, &ui);
    assert!(
        after.iter().any(|t| t == "explorer/alpha"),
        "the worktree's branch: {after:?}"
    );
}

#[test]
fn a_task_picked_up_next_opens_its_session() {
    let mut app = full_app();
    app.task.tasks = vec![task("gh-a-b-1", "waiting one", "github.com/a/b#1")];
    let (mut ui, hits) = on_board(&app);
    let row = hits
        .rect_of(&Target::Task("gh-a-b-1".into()))
        .expect("the task has a row");
    let commands = click(row, &mut ui, &app, &hits);
    assert_eq!(ui.surface, Surface::Session, "the window goes to the work");
    assert_eq!(
        commands.iter().map(|c| c.id()).collect::<Vec<_>>(),
        ["task.open"]
    );
}

#[test]
fn a_task_up_next_shows_where_it_sits_in_the_plan() {
    let mut app = full_app();
    app.task.tasks = vec![
        task("gh-a-b-1", "waiting one", "github.com/a/b#1"),
        task("gh-a-b-2", "waiting two", "github.com/a/b#2"),
    ];
    let (ui, _) = on_board(&app);
    let drawn = texts(&app, &ui);
    assert!(drawn.iter().any(|t| t == "1"), "{drawn:?}");
    assert!(drawn.iter().any(|t| t == "2"), "{drawn:?}");
}

/// The board with two tasks up next, and the filter holding `text`.
fn filtered(text: &str) -> (AppState, Ui) {
    let mut app = full_app();
    let mut second = task("gh-a-b-2", "waiting two", "github.com/a/b#2");
    second.status = "Todo".into();
    second.priority = Some(groove_types::Priority::Low);
    app.task.tasks = vec![task("gh-a-b-1", "waiting one", "github.com/a/b#1"), second];
    let mut ui = Ui {
        surface: Surface::Board,
        ..Ui::default()
    };
    ui.board.filter.set(text);
    (app, ui)
}

#[test]
fn a_bare_word_keeps_the_items_whose_title_holds_it() {
    let (app, ui) = filtered("two");
    let drawn = texts(&app, &ui);
    assert!(drawn.iter().any(|t| t == "waiting two"), "{drawn:?}");
    assert!(!drawn.iter().any(|t| t == "waiting one"), "{drawn:?}");
    assert!(drawn.iter().any(|t| t == "UP NEXT · 1"), "{drawn:?}");
}

#[test]
fn a_field_token_keeps_the_tasks_that_hold_that_value() {
    let (app, ui) = filtered("status:in-progress");
    let drawn = texts(&app, &ui);
    assert!(drawn.iter().any(|t| t == "waiting one"), "{drawn:?}");
    assert!(!drawn.iter().any(|t| t == "waiting two"), "{drawn:?}");
}

#[test]
fn a_filter_a_column_answers_nothing_to_says_so() {
    let (app, ui) = filtered("priority:high");
    let drawn = texts(&app, &ui);
    assert!(
        drawn.iter().any(|t| t == "nothing the filter lets through"),
        "no session has a priority: {drawn:?}"
    );
}

#[test]
fn the_filter_offers_the_fields_it_knows_and_a_pick_lands_in_it() {
    let app = full_app();
    let mut ui = Ui {
        surface: Surface::Board,
        ..Ui::default()
    };
    ui.board.focus();
    ui.board.filter.set("pri");
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let row = hits
        .rect_of(&Target::Offer(0))
        .expect("the field is offered");
    click(row, &mut ui, &app, &hits);
    assert_eq!(ui.board.filter.text(), "priority:");
}

#[test]
fn the_slash_opens_the_filter_and_escape_empties_it_then_leaves() {
    let app = full_app();
    let mut ui = Ui {
        surface: Surface::Board,
        ..Ui::default()
    };
    press(Key::Char('/'), Modifiers::default(), &mut ui, &app);
    assert!(ui.board.typing, "the keyboard is in the filter");
    press(Key::Char('a'), Modifiers::default(), &mut ui, &app);
    assert_eq!(ui.board.filter.text(), "a");
    press(Key::Escape, Modifiers::default(), &mut ui, &app);
    assert!(ui.board.filter.is_empty(), "the first escape clears it");
    assert!(ui.board.typing, "and stays in it");
    press(Key::Escape, Modifiers::default(), &mut ui, &app);
    assert!(!ui.board.typing, "the second leaves it");
}

#[test]
fn the_header_s_new_task_opens_an_explorer() {
    let app = full_app();
    let mut ui = Ui {
        surface: Surface::Board,
        ..Ui::default()
    };
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let button = hits
        .rect_of(&Target::AddTask)
        .expect("the header offers it");
    let commands = click(button, &mut ui, &app, &hits);
    assert_eq!(
        commands.iter().map(|c| c.id()).collect::<Vec<_>>(),
        ["session.open_explorer"]
    );
    assert_eq!(ui.surface, Surface::Session, "the explorer is the work");
}

#[test]
fn enter_takes_the_offer_the_keyboard_stands_on() {
    let mut app = full_app();
    app.task.tasks = vec![task("gh-a-b-1", "waiting one", "github.com/a/b#1")];
    let mut ui = Ui {
        surface: Surface::Board,
        ..Ui::default()
    };
    press(Key::Char('/'), Modifiers::default(), &mut ui, &app);
    for c in "stat".chars() {
        press(Key::Char(c), Modifiers::default(), &mut ui, &app);
    }
    press(Key::Enter, Modifiers::default(), &mut ui, &app);
    assert_eq!(ui.board.filter.text(), "status:", "the field it named");
    press(Key::Enter, Modifiers::default(), &mut ui, &app);
    assert_eq!(
        ui.board.filter.text(),
        "status:In-progress",
        "then the value the board holds"
    );
    assert!(
        crate::views::board::complete::offers(&app, ui.board.filter.text()).is_empty(),
        "a whole token is offered nothing"
    );
}

#[test]
fn an_unfolded_item_holds_no_rule_between_its_own_rows() {
    let app = full_app();
    let (mut ui, hits) = on_board(&app);
    let id = app
        .session
        .open
        .first()
        .map(|open| open.session.id.clone())
        .expect("the fixture has a session");
    let twisty = hits
        .rect_of(&Target::Unfold(id))
        .expect("the item has a twisty");
    click(twisty, &mut ui, &app, &hits);
    let (frame, _) = view(&app, &ui, window(), &mut Fonts::embedded());
    let tokens = crate::tokens::Tokens::new(1.0);
    let styles = crate::style::Styles::new(app.config.theme(), tokens);
    let board = crate::layout::Layout::of(window(), &ui).board;
    let width = (board.w / 3.0).floor();
    let rules = frame.layers()[0]
        .quads
        .iter()
        .filter(|quad| quad.color == styles.line() && quad.rect.h == tokens.hairline)
        .filter(|quad| quad.rect.x >= board.x && quad.rect.x < board.x + width)
        .count();
    assert_eq!(
        rules, 4,
        "the header, the heading, the item's last row, the session under it"
    );
}

#[test]
fn the_pointer_takes_the_light_from_the_row_the_keyboard_stands_on() {
    let app = full_app();
    let mut ui = Ui {
        surface: Surface::Board,
        ..Ui::default()
    };
    ui.board.focus();
    let styles = crate::style::Styles::new(app.config.theme(), crate::tokens::Tokens::new(1.0));
    let lit = |ui: &Ui| {
        let (frame, hits) = view(&app, ui, window(), &mut Fonts::embedded());
        let rows: Vec<f32> = (0..2)
            .filter_map(|at| hits.rect_of(&Target::Offer(at)).map(|rect| rect.y))
            .collect();
        let tall = crate::tokens::Tokens::new(1.0).row;
        let on = frame.layers()[1]
            .quads
            .iter()
            .filter(|quad| quad.color == styles.hover() && quad.rect.h == tall)
            .map(|quad| quad.rect.y)
            .collect::<Vec<f32>>();
        (rows, on)
    };
    let (rows, on) = lit(&ui);
    assert_eq!(on, vec![rows[0]], "the keyboard stands on the first");
    ui.hover = Some(Target::Offer(1));
    let (rows, on) = lit(&ui);
    assert_eq!(on, vec![rows[1]], "the pointer moves it, and only it");
}

#[test]
fn the_header_s_button_is_drawn_with_a_border_around_it() {
    let app = full_app();
    let ui = Ui {
        surface: Surface::Board,
        ..Ui::default()
    };
    let (frame, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let box_ = hits.rect_of(&Target::AddTask).expect("the button");
    let styles = crate::style::Styles::new(app.config.theme(), crate::tokens::Tokens::new(1.0));
    let thin = crate::tokens::Tokens::new(1.0).hairline;
    let edges = frame.layers()[0]
        .quads
        .iter()
        .filter(|quad| quad.color == styles.line())
        .filter(|quad| quad.rect.w == thin || quad.rect.h == thin)
        .filter(|quad| quad.rect.x >= box_.x && quad.rect.right() <= box_.right())
        .count();
    assert_eq!(edges, 4, "one edge a side");
}
