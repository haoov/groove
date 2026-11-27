//! A write the agent waits on: on its session's row, in the review sheet it opens, and the
//! switch that lets it through without asking.

use groove_controllers::{Command, agent};
use groove_gfx::Fonts;
use groove_types::{ApprovalId, SessionId};

use super::agent::{ask, asking, session_ui};
use crate::Ui;
use crate::hit::Target;
use crate::input::{Key, Modifiers};
use crate::tests::{all_texts, click, press, window};

/// Every text the frame drew, over every layer.
fn texts(app: &groove_controllers::AppState, ui: &Ui) -> Vec<String> {
    let (frame, _) = crate::view(app, ui, window(), &mut Fonts::embedded());
    all_texts(&frame)
}

fn committing() -> groove_controllers::AppState {
    let mut one = ask("ap-1", "git_commit", "fix: one");
    one.text = "fix: one\n\nthe body of it".into();
    asking(vec![one])
}

#[test]
fn a_row_that_asks_says_what_it_asks_and_offers_no_buttons() {
    let app = committing();
    let ui = session_ui();
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let id = ApprovalId::new("ap-1");
    assert_eq!(
        hits.rect_of(&Target::Approve(id)),
        None,
        "no answer on the row"
    );
    assert!(
        texts(&app, &ui).iter().any(|one| one == "asks to commit"),
        "the row says what it asks, whole"
    );
    let height = |app: &groove_controllers::AppState| {
        let (_, hits) = crate::view(app, &ui, window(), &mut Fonts::embedded());
        let row = hits.rect_of(&Target::Session(SessionId::new("a")));
        row.expect("the row").h
    };
    assert_eq!(
        height(&app),
        height(&asking(Vec::new())),
        "no taller for it"
    );
}

#[test]
fn a_click_on_a_row_that_asks_opens_the_session_on_its_review() {
    let app = committing();
    let mut ui = Ui::default();
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let row = hits
        .rect_of(&Target::Session(SessionId::new("a")))
        .expect("the row");
    let acted = click(row, &mut ui, &app, &hits);
    let open = groove_controllers::session::Command::Open {
        session: SessionId::new("a"),
    };
    assert_eq!(acted, [Command::Session(open)]);
    assert_eq!(ui.examining(), Some(&ApprovalId::new("ap-1")));
}

#[test]
fn review_shows_the_whole_write_and_its_two_answers() {
    let app = committing();
    let mut ui = session_ui();
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let row = hits
        .rect_of(&Target::Session(SessionId::new("a")))
        .expect("the row");
    click(row, &mut ui, &app, &hits);
    assert_eq!(ui.examining(), Some(&ApprovalId::new("ap-1")));

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
    let mut ui = session_ui();
    ui.examine(id.clone());
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let rect = hits.rect_of(&Target::Refuse(id.clone())).expect("refuse");
    let acted = click(rect, &mut ui, &app, &hits);
    assert_eq!(acted, [Command::Agent(agent::Command::Refuse { id })]);
    assert_eq!(ui.examining(), None);
}

#[test]
fn escape_refuses_the_write_on_the_sheet_and_enter_approves_it() {
    let app = committing();
    let id = ApprovalId::new("ap-1");
    let mut ui = session_ui();
    ui.examine(id.clone());
    let refused = press(Key::Escape, Modifiers::default(), &mut ui, &app);
    let refuse = agent::Command::Refuse { id: id.clone() };
    assert_eq!(refused, [Command::Agent(refuse)]);
    assert_eq!(ui.examining(), None);
    ui.examine(id.clone());
    let approved = press(Key::Enter, Modifiers::default(), &mut ui, &app);
    assert_eq!(approved, [Command::Agent(agent::Command::Approve { id })]);
    assert_eq!(ui.examining(), None);
}

#[test]
fn arriving_on_a_session_that_asks_opens_its_sheet_and_another_s_never_shows() {
    let app = committing();
    let mut ui = session_ui();
    ui.settle(&app);
    assert_eq!(
        ui.examining(),
        Some(&ApprovalId::new("ap-1")),
        "on arriving"
    );
    ui.overlay = None;
    ui.settle(&app);
    assert_eq!(ui.examining(), None, "only on arriving, never while there");
    ui.examine(ApprovalId::new("elsewhere"));
    ui.settle(&app);
    assert_eq!(
        ui.examining(),
        None,
        "another session's write is not shown here"
    );
}

#[test]
fn the_sheet_floats_over_the_middle() {
    let app = committing();
    let mut ui = session_ui();
    ui.examine(ApprovalId::new("ap-1"));
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let sheet = hits.rect_of(&Target::Sheet).expect("the sheet");
    let middle = crate::layout::Layout::of(window(), &ui).agent;
    assert!(middle.contains(sheet.x, sheet.y) && sheet.right() <= middle.right());
}

#[test]
fn the_bar_keeps_no_switch() {
    let app = committing();
    let ui = session_ui();
    let drawn = texts(&app, &ui);
    assert!(
        !drawn.iter().any(|one| one.starts_with("auto-approve")),
        "{drawn:?}"
    );
}
