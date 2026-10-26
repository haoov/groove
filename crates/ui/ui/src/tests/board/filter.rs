//! The board's filter: what it narrows, what it offers, and the header it sits in.

use groove_controllers::AppState;
use groove_gfx::Fonts;

use super::texts;
use crate::hit::Target;
use crate::input::{Key, Modifiers};
use crate::tests::{click, full_app, press, task, typed, window};
use crate::{Surface, Ui, view};

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
    typed("stat", &mut ui, &app);
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
fn the_pointer_takes_the_light_from_the_row_the_keyboard_stands_on() {
    let app = full_app();
    let mut ui = Ui {
        surface: Surface::Board,
        ..Ui::default()
    };
    ui.board.focus();
    let styles = groove_ui_kit::base::style::Styles::new(
        app.config.theme(),
        groove_ui_kit::base::tokens::Tokens::new(1.0),
    );
    let lit = |ui: &Ui| {
        let (frame, hits) = view(&app, ui, window(), &mut Fonts::embedded());
        let rows: Vec<f32> = (0..2)
            .filter_map(|at| hits.rect_of(&Target::Offer(at)).map(|rect| rect.y))
            .collect();
        let tall = groove_ui_kit::base::tokens::Tokens::new(1.0).row;
        let mut on = frame.layers()[1]
            .quads
            .iter()
            .filter(|quad| quad.color == styles.hover() && quad.rect.h == tall)
            .map(|quad| quad.rect.y)
            .collect::<Vec<f32>>();
        on.dedup();
        (rows, on)
    };
    let (rows, on) = lit(&ui);
    assert_eq!(on, vec![rows[0]], "the keyboard stands on the first");
    ui.hover = Some(Target::Offer(1));
    let (rows, on) = lit(&ui);
    assert_eq!(on, vec![rows[1]], "the pointer moves it, and only it");
}

#[test]
fn the_filter_stands_on_glass_at_the_top_middle_and_the_button_beside_it() {
    let app = full_app();
    let mut ui = Ui {
        surface: Surface::Board,
        ..Ui::default()
    };
    let (frame, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let board = crate::layout::Layout::of(window(), &ui).board;
    let button = hits.rect_of(&Target::AddTask).expect("the button");
    let filter = hits.rect_of(&Target::Filter).expect("the filter");
    let glass = &frame.layers()[0].glass;
    let pane = |rect: groove_gfx::Rect| glass.iter().find(|one| one.rect == rect);
    assert!(pane(filter).is_some(), "the filter on its own glass");
    let round = pane(button).expect("the button made of glass");
    assert_eq!(round.radius, button.w / 2.0, "round");
    let centre = (filter.x + filter.w / 2.0 - (board.x + board.w / 2.0)).abs();
    assert!(centre <= 1.0, "the filter in the middle");
    let gap = button.x - filter.right();
    assert!(
        gap > 0.0 && gap <= 8.0,
        "the button just right of it: {gap}"
    );
    assert!(filter.y - board.y < board.h / 8.0, "both at the top");
    let hover = groove_ui_kit::base::style::Styles::new(
        app.config.theme(),
        groove_ui_kit::base::tokens::Tokens::new(1.0),
    )
    .glass_hover();
    let lit = |frame: &groove_gfx::Frame| {
        frame.layers()[0]
            .quads
            .iter()
            .any(|quad| quad.rect == button && quad.color == hover)
    };
    assert!(!lit(&frame), "at rest, glass alone");
    ui.hover = Some(Target::AddTask);
    let (frame, _) = view(&app, &ui, window(), &mut Fonts::embedded());
    assert!(lit(&frame), "the pointer lights the whole glass");
}

#[test]
fn a_live_item_answers_for_the_task_its_session_works() {
    let mut app = full_app();
    let external = groove_types::ExternalId::new("github.com/a/b#1");
    let mut one = task("gh-a-b-1", "already open", external.as_str());
    one.priority = Some(groove_types::Priority::High);
    app.task.tasks = vec![one];
    let kind = groove_types::SessionKind::Task {
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
    let mut ui = Ui {
        surface: Surface::Board,
        ..Ui::default()
    };
    ui.board.filter.set("priority:high");
    let drawn = texts(&app, &ui);
    assert!(drawn.iter().any(|t| t == "LIVE · 1"), "{drawn:?}");

    ui.board.filter.set("priority:low");
    let drawn = texts(&app, &ui);
    assert!(drawn.iter().any(|t| t == "LIVE"), "no count: {drawn:?}");
    assert!(
        drawn.iter().any(|t| t == "nothing the filter lets through"),
        "{drawn:?}"
    );
}

#[test]
fn a_source_can_be_named_and_the_other_one_s_tasks_go() {
    let mut app = full_app();
    let mut notion = task("TASK-1", "from notion", "1f2e3d4c");
    notion.provider = groove_types::ProviderId::Notion;
    app.task.tasks = vec![task("gh-a-b-1", "from github", "github.com/a/b#1"), notion];
    let mut ui = Ui {
        surface: Surface::Board,
        ..Ui::default()
    };
    ui.board.filter.set("provider:notion");
    let drawn = texts(&app, &ui);
    assert!(drawn.iter().any(|t| t == "from notion"), "{drawn:?}");
    assert!(!drawn.iter().any(|t| t == "from github"), "{drawn:?}");
}
