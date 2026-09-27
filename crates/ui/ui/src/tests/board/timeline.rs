//! The band under the columns: what stands in the horizon, and what folds it away.

use groove_controllers::AppState;
use groove_gfx::Fonts;
use groove_types::Timestamp;

use super::texts;
use crate::base::hit::Target;
use crate::tests::{click, full_app, task, window};
use crate::{Surface, Ui, view};

/// The board with one task running from last week to next.
fn running() -> (AppState, Ui) {
    let mut app = full_app();
    let today = Timestamp::now().day();
    let mut one = task("gh-a-b-1", "waiting one", "github.com/a/b#1");
    one.dates.start = Some(today.plus_days(-2));
    one.dates.due = Some(today.plus_days(3));
    app.task.tasks = vec![one];
    let ui = Ui {
        surface: Surface::Board,
        ..Ui::default()
    };
    (app, ui)
}

#[test]
fn the_band_names_its_horizon_and_draws_what_falls_in_it() {
    let (app, ui) = running();
    let drawn = texts(&app, &ui);
    assert!(drawn.iter().any(|t| t == "TIMELINE"), "{drawn:?}");
    assert!(
        drawn.iter().any(|t| t.starts_with("4 weeks · ")),
        "{drawn:?}"
    );
    assert_eq!(
        drawn.iter().filter(|t| *t == "waiting one").count(),
        2,
        "its row and its bar: {drawn:?}"
    );
}

#[test]
fn a_board_with_no_dates_in_the_horizon_folds_the_band_away() {
    let mut app = full_app();
    app.task.tasks = vec![task("gh-a-b-1", "waiting one", "github.com/a/b#1")];
    let ui = Ui {
        surface: Surface::Board,
        ..Ui::default()
    };
    let tokens = crate::base::tokens::Tokens::new(1.0);
    let board = crate::layout::Layout::of(window(), &ui).board;
    let band = crate::views::board::bands(&tokens, &app, &ui, board).timeline;
    assert_eq!(band.h, tokens.header, "its own bar and nothing more");
    let drawn = texts(&app, &ui);
    assert_eq!(
        drawn.iter().filter(|t| *t == "waiting one").count(),
        1,
        "the row alone: {drawn:?}"
    );
}

#[test]
fn the_band_s_bar_folds_it_away_and_gives_the_room_to_the_columns() {
    let (app, mut ui) = running();
    let tokens = crate::base::tokens::Tokens::new(1.0);
    let board = crate::layout::Layout::of(window(), &ui).board;
    let bands = crate::views::board::bands(&tokens, &app, &ui, board);
    let (open, band) = (bands.columns, bands.timeline);
    assert_eq!(band.h, tokens.band, "the whole of it stands");

    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let bar = hits.rect_of(&Target::Timeline).expect("the band's own bar");
    assert!(click(bar, &mut ui, &app, &hits).is_empty(), "no command");
    assert!(ui.board.shut);
    let bands = crate::views::board::bands(&tokens, &app, &ui, board);
    let (shut, band) = (bands.columns, bands.timeline);
    assert_eq!(band.h, tokens.header);
    assert!(shut.h > open.h, "the columns take what it gave up");
}

#[test]
fn a_sideways_turn_over_the_band_carries_it_through_time() {
    let (app, mut ui) = running();
    let tokens = crate::base::tokens::Tokens::new(1.0);
    let board = crate::layout::Layout::of(window(), &ui).board;
    let band = crate::views::board::bands(&tokens, &app, &ui, board).timeline;
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let before = texts(&app, &ui)
        .into_iter()
        .find(|t| t.starts_with("4 weeks · "))
        .expect("the horizon");

    crate::input::handle(
        crate::input::Input::Scroll {
            x: band.x + 10.0,
            y: band.y + band.h / 2.0,
            delta: crate::input::Delta::Lines {
                across: -7.0,
                down: 0.0,
            },
        },
        &mut ui,
        &app,
        &hits,
        window(),
    );
    assert_eq!(ui.board.horizon, 7, "a week later");
    let after = texts(&app, &ui)
        .into_iter()
        .find(|t| t.starts_with("4 weeks · "))
        .expect("the horizon");
    assert_ne!(before, after, "the band says where it now stands");
}

#[test]
fn a_turn_straight_down_over_the_band_carries_nothing() {
    let (app, mut ui) = running();
    let tokens = crate::base::tokens::Tokens::new(1.0);
    let board = crate::layout::Layout::of(window(), &ui).board;
    let band = crate::views::board::bands(&tokens, &app, &ui, board).timeline;
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    crate::input::handle(
        crate::input::Input::Scroll {
            x: band.x + 10.0,
            y: band.y + band.h / 2.0,
            delta: crate::input::Delta::Lines {
                across: 0.0,
                down: -3.0,
            },
        },
        &mut ui,
        &app,
        &hits,
        window(),
    );
    assert_eq!(ui.board.horizon, 0, "time stands where it stood");
}

#[test]
fn a_drag_on_the_band_s_edge_makes_it_taller() {
    let (app, mut ui) = running();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let grab = hits
        .rect_of(&Target::Split(crate::layout::Edge::Band))
        .expect("the band offers its edge");
    let before = ui.split.band;
    crate::tests::pressed(grab.x + 10.0, grab.y + grab.h / 2.0, &mut ui, &app, &hits);
    crate::tests::drag_at(grab.x + 10.0, grab.y - 60.0, &mut ui, &app, &hits);
    assert!(ui.split.band > before, "the band took the room");
}

#[test]
fn the_bar_under_the_pointer_names_its_task() {
    let (app, mut ui) = running();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let bar = hits
        .rect_of(&Target::Bar("gh-a-b-1".into()))
        .expect("the task has a bar");
    crate::input::hover(&mut ui, &hits, bar.x + 2.0, bar.y + 2.0);
    let (frame, _) = view(&app, &ui, window(), &mut Fonts::embedded());
    let named = frame
        .layers()
        .iter()
        .flat_map(|layer| layer.texts.iter())
        .filter(|t| t.text == "waiting one")
        .count();
    assert_eq!(named, 3, "its row, its bar, and the tip under the pointer");
}

#[test]
fn a_sweep_too_small_for_a_day_is_kept_for_the_next_one() {
    let (app, mut ui) = running();
    let tokens = crate::base::tokens::Tokens::new(1.0);
    let board = crate::layout::Layout::of(window(), &ui).board;
    let band = crate::views::board::bands(&tokens, &app, &ui, board).timeline;
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let sweep = |across: f32, ui: &mut Ui, hits: &crate::base::hit::Hits| {
        crate::input::handle(
            crate::input::Input::Scroll {
                x: band.x + 10.0,
                y: band.y + band.h / 2.0,
                delta: crate::input::Delta::Pixels { across, down: 0.0 },
            },
            ui,
            &app,
            hits,
            window(),
        );
    };
    let half = crate::base::tokens::DAY_PIXELS / 2.0 + 1.0;
    sweep(-half, &mut ui, &hits);
    assert_eq!(ui.board.horizon, 0, "not a day yet");
    sweep(-half, &mut ui, &hits);
    assert_eq!(ui.board.horizon, 1, "the two halves make it");
}

#[test]
fn a_bar_keeps_its_row_wherever_the_horizon_stands() {
    let mut app = full_app();
    let today = Timestamp::now().day();
    let mut one = task("gh-a-b-1", "waiting one", "github.com/a/b#1");
    one.dates.start = Some(today.plus_days(-3));
    one.dates.due = Some(today.plus_days(1));
    let mut two = task("gh-a-b-2", "waiting two", "github.com/a/b#2");
    two.dates.start = Some(today.plus_days(-1));
    two.dates.due = Some(today.plus_days(6));
    let mut three = task("gh-a-b-3", "waiting three", "github.com/a/b#3");
    three.dates.start = Some(today.plus_days(8));
    three.dates.due = Some(today.plus_days(10));
    app.task.tasks = vec![one, two, three];
    let mut ui = Ui {
        surface: Surface::Board,
        ..Ui::default()
    };
    let rows = |ui: &Ui| {
        let (_, hits) = view(&app, ui, window(), &mut Fonts::embedded());
        ["gh-a-b-1", "gh-a-b-2", "gh-a-b-3"]
            .map(|short_id| hits.rect_of(&Target::Bar(short_id.into())).map(|one| one.y))
    };
    let before = rows(&ui);
    assert_ne!(before[0], before[1], "two bars that overlap stack");
    ui.board.horizon = 5;
    let after = rows(&ui);
    for (was, now) in before.iter().zip(after.iter()) {
        if let (Some(was), Some(now)) = (was, now) {
            assert_eq!(was, now, "the row a bar keeps");
        }
    }
}
