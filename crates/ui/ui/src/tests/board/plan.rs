//! Up next: the plan's own order, and the drag that changes it.

use groove_controllers::AppState;
use groove_gfx::Fonts;
use groove_types::ExternalId;

use super::{on_board, texts};
use crate::hit::Target;
use crate::tests::{click, drag_at, full_app, pressed, release, task, window};
use crate::{Surface, Ui, view};

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
fn a_task_up_next_is_moved_by_the_mark_of_its_source() {
    let mut app = full_app();
    app.task.tasks = vec![task("gh-a-b-1", "waiting one", "github.com/a/b#1")];
    let (ui, hits) = on_board(&app);
    let (frame, _) = view(&app, &ui, window(), &mut Fonts::embedded());
    let handle = hits
        .rect_of(&Target::Place(ExternalId::new("github.com/a/b#1")))
        .expect("the drag handle");
    let mark = frame.layers()[0]
        .icons
        .iter()
        .find(|one| one.icon == groove_gfx::Icon::Github)
        .expect("the source's mark");
    assert_eq!(mark.rect, handle, "the mark is the handle");
}

/// The board with three tasks up next, nothing running.
fn waiting() -> (AppState, Ui) {
    let mut app = full_app();
    app.session.open.clear();
    app.session.living.clear();
    app.session.selected = None;
    app.task.tasks = vec![
        task("gh-a-b-1", "waiting one", "github.com/a/b#1"),
        task("gh-a-b-2", "waiting two", "github.com/a/b#2"),
        task("gh-a-b-3", "waiting three", "github.com/a/b#3"),
    ];
    (app, Ui::default())
}

/// The order Up next draws its titles in.
fn up_next(app: &AppState, ui: &Ui) -> Vec<String> {
    texts(app, ui)
        .into_iter()
        .filter(|text| text.starts_with("waiting"))
        .collect()
}

#[test]
fn up_next_stands_in_the_order_the_plan_gives_it() {
    let (mut app, ui) = waiting();
    app.task.plan = vec![
        groove_controllers::task_service::Placed::now(ExternalId::new("github.com/a/b#3")),
        groove_controllers::task_service::Placed::now(ExternalId::new("github.com/a/b#2")),
    ];
    assert_eq!(
        up_next(&app, &ui),
        ["waiting three", "waiting two", "waiting one"],
        "the placed ones first, then the rest"
    );
}

#[test]
fn a_task_dragged_by_its_place_lands_where_it_was_dropped() {
    let (app, mut ui) = waiting();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let third = ExternalId::new("github.com/a/b#3");
    let handle = hits
        .rect_of(&Target::Place(third.clone()))
        .expect("the row carries its place");
    let first = hits
        .rect_of(&Target::Place(ExternalId::new("github.com/a/b#1")))
        .expect("the first row too");
    assert!(
        pressed(handle.x + 1.0, handle.y + 1.0, &mut ui, &app, &hits).is_empty(),
        "the press takes hold and asks for nothing"
    );
    assert_eq!(ui.carried(), Some(&third));
    assert!(
        ui.pointing(),
        "so the window sends what the pointer does next"
    );

    drag_at(first.x + 1.0, first.y + 1.0, &mut ui, &app, &hits);
    assert_eq!(ui.board.drop, Some(0), "it would land on the first line");
    let commands = release(&mut ui, &app, &hits);
    assert_eq!(
        commands,
        [groove_controllers::Command::Task(
            groove_controllers::task::Command::Plan(groove_controllers::task::Landing {
                external_id: third,
                before: Some(ExternalId::new("github.com/a/b#1")),
                later: false,
            })
        )]
    );
    assert!(ui.carried().is_none(), "the drag is spent");
}

#[test]
fn a_task_dropped_under_the_divider_is_asked_for_later() {
    let (app, mut ui) = waiting();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let first = ExternalId::new("github.com/a/b#1");
    let handle = hits
        .rect_of(&Target::Place(first.clone()))
        .expect("the row carries its place");
    pressed(handle.x + 1.0, handle.y + 1.0, &mut ui, &app, &hits);
    let tokens = groove_ui_kit::base::tokens::Tokens::new(1.0);
    let board = crate::layout::Layout::of(window(), &ui).board;
    let body = crate::views::board::columns(&tokens, board);
    let width = (body.w / 3.0).floor();
    drag_at(
        body.x + width + 10.0,
        body.bottom() - 1.0,
        &mut ui,
        &app,
        &hits,
    );
    let commands = release(&mut ui, &app, &hits);
    assert_eq!(
        commands,
        [groove_controllers::Command::Task(
            groove_controllers::task::Command::Plan(groove_controllers::task::Landing {
                external_id: first,
                before: None,
                later: true,
            })
        )],
        "the end of the later side"
    );
}

#[test]
fn a_task_up_next_shows_no_time() {
    let mut app = full_app();
    app.task.tasks = vec![task("gh-a-b-1", "waiting one", "github.com/a/b#1")];
    let (ui, _) = on_board(&app);
    let drawn = texts(&app, &ui);
    assert!(!drawn.iter().any(|t| t.contains("4h")), "{drawn:?}");
}

#[test]
fn a_task_s_priority_takes_its_level_s_colour() {
    let mut app = full_app();
    let mut high = task("gh-a-b-1", "urgent one", "github.com/a/b#1");
    high.priority = Some(groove_types::Priority::High);
    let mut low = task("gh-a-b-2", "calm one", "github.com/a/b#2");
    low.priority = Some(groove_types::Priority::Low);
    app.task.tasks = vec![high, low];
    let (ui, _) = on_board(&app);
    let (frame, _) = view(&app, &ui, window(), &mut Fonts::embedded());
    let styles = groove_ui_kit::base::style::Styles::new(
        app.config.theme(),
        groove_ui_kit::base::tokens::Tokens::new(1.0),
    );
    let colour = |word: &str| {
        let texts = &frame.layers()[0].texts;
        texts.iter().find(|t| t.text == word).map(|t| t.style.color)
    };
    use groove_ui_kit::base::style::Role;
    assert_eq!(colour("high"), Some(styles.color(Role::Bad)));
    assert_eq!(colour("low"), Some(styles.color(Role::Ok)));
}

#[test]
fn a_task_up_next_wears_the_mark_of_its_source() {
    let mut app = full_app();
    let mut notion = task("TASKS2-1", "from notion", "notion-page");
    notion.provider = groove_types::ProviderId::Notion;
    app.task.tasks = vec![task("gh-a-b-1", "from github", "github.com/a/b#1"), notion];
    let (ui, _) = on_board(&app);
    let (frame, _) = view(&app, &ui, window(), &mut Fonts::embedded());
    let icons: Vec<groove_gfx::Icon> = frame.layers()[0].icons.iter().map(|i| i.icon).collect();
    assert!(icons.contains(&groove_gfx::Icon::Github), "{icons:?}");
    assert!(icons.contains(&groove_gfx::Icon::Notion), "{icons:?}");
}
