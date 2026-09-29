//! What an item says when it needs the user, and where it stands because of it.

use groove_controllers::AppState;
use groove_gfx::Fonts;
use groove_types::{Attention, Day, ExternalId, Timestamp};

use super::texts;
use crate::hit::Target;
use crate::tests::{full_app, task, window};
use crate::{Surface, Ui, view};

/// The board with two tasks up next, the second of them overdue, neither ranked.
fn overdue() -> (AppState, Ui) {
    let mut app = full_app();
    app.session.open.clear();
    app.session.living.clear();
    app.session.selected = None;
    let mut late = task("gh-a-b-2", "waiting two", "github.com/a/b#2");
    late.dates.due = Some(Timestamp::now().day().plus_days(-3));
    app.task.tasks = vec![task("gh-a-b-1", "waiting one", "github.com/a/b#1"), late];
    for task in &mut app.task.tasks {
        task.priority = None;
        task.project = None;
    }
    app.task.attention.insert(
        ExternalId::new("github.com/a/b#2"),
        vec![Attention::Overdue { by_days: 3 }],
    );
    let ui = Ui {
        surface: Surface::Board,
        ..Ui::default()
    };
    (app, ui)
}

#[test]
fn an_item_that_needs_the_user_says_why_under_its_title() {
    let (app, ui) = overdue();
    let drawn = texts(&app, &ui);
    assert!(drawn.iter().any(|t| t == "overdue 3d"), "{drawn:?}");
}

#[test]
fn an_item_that_needs_the_user_stands_over_the_ones_that_do_not() {
    let (app, ui) = overdue();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let at = |short_id: &str| {
        hits.rect_of(&Target::Task(short_id.into()))
            .expect("the task has a row")
            .y
    };
    assert!(at("gh-a-b-2") < at("gh-a-b-1"), "the overdue one floats up");
}

#[test]
fn a_row_that_says_why_stands_taller_than_one_that_does_not() {
    let (app, ui) = overdue();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let tall = |short_id: &str| {
        hits.rect_of(&Target::Task(short_id.into()))
            .expect("the task has a row")
            .h
    };
    assert!(tall("gh-a-b-2") > tall("gh-a-b-1"), "one line more");
}

#[test]
fn a_ranked_row_carries_its_priority_on_a_second_line() {
    let (mut app, ui) = overdue();
    app.task.tasks[0].priority = Some(groove_types::Priority::Low);
    let (frame, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let row = hits
        .rect_of(&Target::Task("gh-a-b-1".into()))
        .expect("the task has a row");
    let texts = &frame.layers()[0].texts;
    let at = |word: &str| texts.iter().find(|t| t.text == word).expect(word).y;
    assert!(at("low") > at("waiting one"), "under the title");
    assert!(at("low") < row.bottom(), "inside its row");
}

#[test]
fn a_row_names_its_project_before_its_priority() {
    let (mut app, ui) = overdue();
    app.task.tasks[0].priority = Some(groove_types::Priority::Low);
    app.task.tasks[0].project = Some("Platform".into());
    let (frame, _) = view(&app, &ui, window(), &mut Fonts::embedded());
    let texts = &frame.layers()[0].texts;
    let at = |word: &str| texts.iter().find(|t| t.text == word).expect(word);
    assert_eq!(at("Platform").y, at("low").y, "on the second line");
    assert!(at("Platform").x < at("low").x, "before the badge");
}

#[test]
fn the_rail_s_board_row_counts_what_needs_the_user() {
    let (app, ui) = overdue();
    let drawn = texts(&app, &ui);
    assert!(drawn.iter().any(|t| t == "1"), "one item asks: {drawn:?}");
}

#[test]
fn a_day_well_before_the_due_date_asks_for_nothing() {
    let mut app = full_app();
    let mut soon = task("gh-a-b-1", "waiting one", "github.com/a/b#1");
    soon.dates.due = Some(Day::parse("2099-01-01").expect("a day"));
    app.task.tasks = vec![soon];
    let ui = Ui {
        surface: Surface::Board,
        ..Ui::default()
    };
    let drawn = texts(&app, &ui);
    assert!(!drawn.iter().any(|t| t.contains("due")), "{drawn:?}");
}
