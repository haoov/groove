//! The rail's own log: what it says, what folds it, and what narrows it.

use groove_gfx::Fonts;
use groove_types::{SessionId, TimelineEvent, TimelineKind, Timestamp};

use crate::hit::Target;
use crate::tests::{app, click, window};
use crate::{Ui, view};

fn line(session: &str, at: i64, kind: TimelineKind, subject: &str) -> TimelineEvent {
    TimelineEvent {
        session: SessionId::new(session),
        at: Timestamp::new(at),
        kind,
        subject: subject.to_string(),
        payload: serde_json::Value::Null,
    }
}

/// The fixture's two sessions, with a line each.
fn fed() -> groove_controllers::AppState {
    let mut app = app();
    app.session.feed = vec![
        line("b", 20, TimelineKind::Push, "fix/two"),
        line("a", 10, TimelineKind::Commit, "fix: one"),
    ];
    app
}

/// Every text the rail drew.
fn in_rail(app: &groove_controllers::AppState, ui: &Ui) -> Vec<String> {
    let (frame, _) = view(app, ui, window(), &mut Fonts::embedded());
    let rail = crate::layout::Layout::of(window(), ui).rail;
    frame.layers()[0]
        .texts
        .iter()
        .filter(|run| run.x < rail.right())
        .map(|run| run.text.clone())
        .collect()
}

#[test]
fn the_feed_says_what_each_session_did() {
    let drawn = in_rail(&fed(), &Ui::default());
    assert!(drawn.iter().any(|one| one == "FEED"), "{drawn:?}");
    assert!(
        drawn.iter().any(|one| one == "commit fix: one"),
        "{drawn:?}"
    );
    assert!(drawn.iter().any(|one| one == "push fix/two"), "{drawn:?}");
}

#[test]
fn a_feed_with_nothing_in_it_says_so() {
    let drawn = in_rail(&app(), &Ui::default());
    assert!(drawn.iter().any(|one| one == "nothing yet"), "{drawn:?}");
}

#[test]
fn the_heading_folds_the_feed_away() {
    let app = fed();
    let mut ui = Ui::default();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let head = hits.rect_of(&Target::Feed).expect("the heading");
    click(head, &mut ui, &app, &hits);
    assert!(ui.rail.folded);
    let drawn = in_rail(&app, &ui);
    assert!(drawn.iter().any(|one| one == "FEED"), "its own row stays");
    assert!(
        !drawn.iter().any(|one| one == "commit fix: one"),
        "and the lines go: {drawn:?}"
    );
}

#[test]
fn the_feed_narrows_to_the_session_in_hand() {
    let app = fed();
    let mut ui = Ui::default();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let scope = hits.rect_of(&Target::FeedScope).expect("the scope");
    click(scope, &mut ui, &app, &hits);
    assert!(ui.rail.mine);
    let drawn = in_rail(&app, &ui);
    assert!(
        drawn.iter().any(|one| one == "commit fix: one"),
        "the selected session's own: {drawn:?}"
    );
    assert!(
        !drawn.iter().any(|one| one == "push fix/two"),
        "and no other's: {drawn:?}"
    );
}

#[test]
fn the_rows_lose_the_room_the_feed_takes() {
    let app = fed();
    let open = Ui::default();
    let mut folded = Ui::default();
    folded.rail.folded = true;
    let room = |ui: &Ui| {
        let (_, hits) = view(&app, ui, window(), &mut Fonts::embedded());
        hits.rect_of(&Target::Feed).expect("the heading").y
    };
    assert!(
        room(&folded) > room(&open),
        "folded, the rows have the rail to themselves"
    );
}

#[test]
fn the_feed_is_dragged_taller_and_the_rows_give_way() {
    let app = fed();
    let mut ui = Ui::default();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let edge = hits
        .rect_of(&Target::Split(crate::layout::Edge::Feed))
        .expect("the feed's own edge");
    let before = crate::layout::Layout::of(window(), &ui).feed;

    crate::tests::pressed(edge.x + 4.0, edge.y + 2.0, &mut ui, &app, &hits);
    crate::tests::drag_at(edge.x + 4.0, edge.y - 120.0, &mut ui, &app, &hits);
    crate::tests::release(&mut ui, &app, &hits);

    let after = crate::layout::Layout::of(window(), &ui).feed;
    assert!(after.h > before.h, "{} against {}", after.h, before.h);
    assert!(after.y < before.y, "it grows upwards into the rows");
}

#[test]
fn the_feed_keeps_its_own_minimum() {
    let app = fed();
    let mut ui = Ui::default();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let edge = hits
        .rect_of(&Target::Split(crate::layout::Edge::Feed))
        .expect("the feed's own edge");

    crate::tests::pressed(edge.x + 4.0, edge.y + 2.0, &mut ui, &app, &hits);
    crate::tests::drag_at(
        edge.x + 4.0,
        window().size.height as f32,
        &mut ui,
        &app,
        &hits,
    );
    crate::tests::release(&mut ui, &app, &hits);
    assert_eq!(ui.split.feed, crate::tokens::FEED_MIN);
}

#[test]
fn a_folded_feed_is_its_heading_alone() {
    let mut ui = Ui::default();
    ui.rail.folded = true;
    let band = crate::layout::Layout::of(window(), &ui).feed;
    assert_eq!(band.h, crate::Tokens::new(1.0).row);
}
