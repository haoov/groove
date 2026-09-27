//! The rail's own log: what it says, what folds it, and what narrows it.

use groove_gfx::Fonts;
use groove_types::{SessionId, TimelineEvent, TimelineKind, Timestamp};

use crate::base::hit::Target;
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
    for one in ["commit", "fix: one", "push", "fix/two"] {
        assert!(drawn.iter().any(|drawn| drawn == one), "{one}: {drawn:?}");
    }
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
        !drawn.iter().any(|one| one == "fix: one"),
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
        drawn.iter().any(|one| one == "fix: one"),
        "the selected session's own: {drawn:?}"
    );
    assert!(
        !drawn.iter().any(|one| one == "fix/two"),
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
    assert_eq!(ui.split.feed, crate::base::tokens::FEED_MIN);
}

#[test]
fn a_folded_feed_is_its_heading_alone() {
    let mut ui = Ui::default();
    ui.rail.folded = true;
    let band = crate::layout::Layout::of(window(), &ui).feed;
    assert_eq!(band.h, crate::Tokens::new(1.0).row);
}

#[test]
fn a_job_in_flight_stands_at_the_top_of_the_feed() {
    let mut app = fed();
    app.begin("committing");
    let drawn = in_rail(&app, &Ui::default());
    let job = drawn.iter().position(|one| one == "committing");
    let done = drawn.iter().position(|one| one == "push");
    assert!(job.is_some(), "{drawn:?}");
    assert!(job < done, "what runs stands above what is over: {drawn:?}");
}

#[test]
fn an_error_is_a_line_of_the_feed() {
    let mut app = fed();
    app.failed(groove_types::Error::internal("no such branch"));
    let drawn = in_rail(&app, &Ui::default());
    let bad = drawn.iter().position(|one| one == "no such branch");
    let old = drawn.iter().position(|one| one == "push");
    assert!(bad.is_some(), "{drawn:?}");
    assert!(drawn.iter().any(|one| one == "error"), "{drawn:?}");
    assert!(bad < old, "the newest first: {drawn:?}");
}

#[test]
fn an_error_is_drawn_in_the_colour_of_a_failure() {
    let mut app = fed();
    app.failed(groove_types::Error::internal("no such branch"));
    let (frame, _) = view(&app, &Ui::default(), window(), &mut Fonts::embedded());
    let bad = frame.layers()[0]
        .texts
        .iter()
        .find(|run| run.text == "error")
        .expect("the error");
    let tokens = crate::Tokens::new(1.0);
    let styles = crate::base::style::Styles::new(groove_types::ThemeName::default(), tokens);
    assert_eq!(bad.style.color, styles.color(crate::base::style::Role::Bad));
}

#[test]
fn what_a_job_says_is_a_line_of_the_feed() {
    let mut app = fed();
    app.say("main is behind origin");
    let drawn = in_rail(&app, &Ui::default());
    assert!(
        drawn.iter().any(|one| one == "main is behind origin"),
        "{drawn:?}"
    );
}

#[test]
fn a_job_and_an_error_are_said_in_the_feed_and_nowhere_else() {
    let mut app = fed();
    app.begin("committing");
    app.failed(groove_types::Error::internal("no such branch"));
    let ui = Ui::default();
    let (frame, _) = view(&app, &ui, window(), &mut Fonts::embedded());
    let feed = crate::layout::Layout::of(window(), &ui).feed;
    let body = feed.y + crate::Tokens::new(1.0).row;
    for text in ["committing", "error", "no such branch"] {
        let drawn: Vec<&groove_gfx::TextRun> = frame.layers()[0]
            .texts
            .iter()
            .filter(|run| run.text == text)
            .collect();
        assert_eq!(drawn.len(), 1, "{text} is drawn once");
        assert!(drawn[0].y >= body, "{text} stands in the feed's own body");
    }
}

#[test]
fn a_line_takes_the_user_to_the_session_it_belongs_to() {
    let app = fed();
    let mut ui = Ui::default();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let target = Target::FeedLine(SessionId::new("b"));
    let line = hits.rect_of(&target).expect("the line");
    let acted = crate::tests::click(line, &mut ui, &app, &hits);
    assert_eq!(ui.surface, crate::Surface::Session);
    assert!(
        acted.iter().any(|one| matches!(
            one,
            groove_controllers::Command::Session(
                groove_controllers::session::Command::Open { session }
            ) if session.as_str() == "b"
        )),
        "{acted:?}"
    );
}

#[test]
fn an_error_and_a_job_take_the_pointer_nowhere() {
    let mut app = fed();
    app.begin("committing");
    app.failed(groove_types::Error::internal("no such branch"));
    let ui = Ui::default();
    let (frame, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    for text in ["committing", "no such branch"] {
        let run = frame.layers()[0]
            .texts
            .iter()
            .find(|run| run.text == text)
            .expect(text);
        let under = hits.at(run.x + 1.0, run.y + 1.0);
        assert!(
            !matches!(under, Some(Target::FeedLine(_))),
            "{text} leads to {under:?}"
        );
    }
}

#[test]
fn the_act_is_bolder_than_its_subject() {
    let app = fed();
    let (frame, _) = view(&app, &Ui::default(), window(), &mut Fonts::embedded());
    let style = |text: &str| {
        frame.layers()[0]
            .texts
            .iter()
            .find(|run| run.text == text)
            .map(|run| run.style)
            .expect(text)
    };
    let act = style("commit");
    let subject = style("fix: one");
    assert_eq!(act.weight, groove_gfx::Weight::Bold);
    assert_eq!(subject.weight, groove_gfx::Weight::Regular);
    assert_ne!(act.color, subject.color);
}

#[test]
fn every_act_starts_at_the_same_column() {
    let mut app = fed();
    app.session.feed[0].at = Timestamp::new(0);
    let (frame, _) = view(&app, &Ui::default(), window(), &mut Fonts::embedded());
    let at = |text: &str| {
        frame.layers()[0]
            .texts
            .iter()
            .find(|run| run.text == text)
            .map(|run| run.x)
            .expect(text)
    };
    assert_eq!(at("push"), at("commit"), "the age's column is fixed");
    assert_eq!(at("push"), at("fix/two"), "and the subject stands under it");
}
