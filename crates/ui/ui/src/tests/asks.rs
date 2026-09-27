//! A write the agent waits on: on its session's row, in the review sheet, and the
//! switch that lets it through without asking.

use groove_controllers::{Command, agent};
use groove_gfx::Fonts;
use groove_types::{ApprovalId, SessionId};

use super::bar::{ask, asking, session_ui};
use crate::Ui;
use crate::base::hit::Target;
use crate::input::{Key, Modifiers};
use crate::tests::{click, press, window};

/// Every text the frame drew, over every layer.
fn texts(app: &groove_controllers::AppState, ui: &Ui) -> Vec<String> {
    let (frame, _) = crate::view(app, ui, window(), &mut Fonts::embedded());
    frame
        .layers()
        .iter()
        .flat_map(|layer| layer.texts.iter().map(|run| run.text.clone()))
        .collect()
}

fn committing() -> groove_controllers::AppState {
    let mut one = ask("ap-1", "git_commit", "fix: one");
    one.text = "fix: one\n\nthe body of it".into();
    asking(vec![one])
}

#[test]
fn a_write_waiting_stands_on_its_row_with_approve_and_review() {
    let app = committing();
    let ui = session_ui();
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let id = ApprovalId::new("ap-1");
    let approve = hits.rect_of(&Target::Approve(id.clone())).expect("approve");
    let review = hits.rect_of(&Target::Examine(id)).expect("review");
    let rail = crate::layout::Layout::of(window(), &ui).rail;
    assert!(
        rail.contains(approve.x, approve.y),
        "on the rail, not the bar"
    );
    assert!(approve.right() <= review.x, "approve, then review");
    assert!(
        texts(&app, &ui).iter().any(|one| one == "asks to commit"),
        "the row says what it asks, whole"
    );
}

#[test]
fn approving_from_the_row_asks_the_controller_to_run_it() {
    let app = committing();
    let mut ui = session_ui();
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let id = ApprovalId::new("ap-1");
    let rect = hits.rect_of(&Target::Approve(id.clone())).expect("approve");
    let acted = click(rect, &mut ui, &app, &hits);
    assert_eq!(acted, [Command::Agent(agent::Command::Approve { id })]);
}

#[test]
fn review_shows_the_whole_write_and_its_two_answers() {
    let app = committing();
    let mut ui = session_ui();
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let id = ApprovalId::new("ap-1");
    let rect = hits.rect_of(&Target::Examine(id.clone())).expect("review");
    assert!(click(rect, &mut ui, &app, &hits).is_empty());
    assert_eq!(ui.examining(), Some(&id));

    let drawn = texts(&app, &ui);
    for said in [
        "asks to commit",
        "fix: one",
        "the body of it",
        "Approve",
        "Refuse",
    ] {
        assert!(drawn.iter().any(|one| one == said), "{said}: {drawn:?}");
    }
}

#[test]
fn refusing_from_the_sheet_drops_the_write_and_puts_the_sheet_away() {
    let app = committing();
    let id = ApprovalId::new("ap-1");
    let mut ui = Ui {
        overlay: Some(crate::Overlay::Examining(id.clone())),
        ..session_ui()
    };
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let rect = hits.rect_of(&Target::Refuse(id.clone())).expect("refuse");
    let acted = click(rect, &mut ui, &app, &hits);
    assert_eq!(acted, [Command::Agent(agent::Command::Refuse { id })]);
    assert_eq!(ui.examining(), None);
}

#[test]
fn escape_puts_the_sheet_away_without_deciding() {
    let app = committing();
    let mut ui = Ui {
        overlay: Some(crate::Overlay::Examining(ApprovalId::new("ap-1"))),
        ..session_ui()
    };
    let acted = press(Key::Escape, Modifiers::default(), &mut ui, &app);
    assert!(acted.is_empty(), "nothing decided: {acted:?}");
    assert_eq!(ui.examining(), None);
}

#[test]
fn the_bar_keeps_its_skills_and_switches_auto_approve() {
    let app = committing();
    let mut ui = session_ui();
    assert!(texts(&app, &ui).iter().any(|one| one == "skills"));
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let session = SessionId::new("a");
    let rect = hits
        .rect_of(&Target::AutoApprove(session.clone()))
        .expect("the switch");
    let acted = click(rect, &mut ui, &app, &hits);
    assert_eq!(
        acted,
        [Command::Agent(agent::Command::AutoApprove {
            session,
            on: true
        })]
    );
}

#[test]
fn the_answers_stand_on_a_line_of_their_own_under_the_ask() {
    let app = committing();
    let ui = session_ui();
    let (frame, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let approve = hits
        .rect_of(&Target::Approve(ApprovalId::new("ap-1")))
        .expect("approve");
    let said = frame
        .layers()
        .iter()
        .flat_map(|layer| layer.texts.iter())
        .find(|run| run.text == "asks to commit")
        .expect("the ask");
    assert!(approve.y > said.y, "the answers are under the ask");
    assert!(
        (approve.x - said.x).abs() <= crate::base::tokens::Tokens::new(1.0).sm,
        "and start where it starts"
    );
    let height = |app: &groove_controllers::AppState| {
        let (_, hits) = crate::view(app, &ui, window(), &mut Fonts::embedded());
        hits.rect_of(&Target::Session(SessionId::new("a")))
            .expect("the row")
            .h
    };
    assert!(
        height(&app) > height(&asking(Vec::new())),
        "the row grows while it asks"
    );
}
