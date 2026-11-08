//! A cluster write under review: its change drawn as the diff draws it, approved only from the sheet.

use groove_controllers::workspace_service::review;
use groove_controllers::{AppState, Command, agent};
use groove_gfx::Fonts;
use groove_types::{ApprovalId, DiffView};

use super::bar::{ask, asking, session_ui};
use crate::hit::Target;
use crate::input::{Key, Modifiers};
use crate::tests::{all_texts, click, press, window};
use crate::{Focus, Ui};

fn patching() -> AppState {
    let mut one = ask("ap-1", "kube_patch", "kind · paxone/deployments/api");
    one.reviewed = true;
    let mut app = asking(vec![one]);
    let before = "spec:\n  replicas: 1\n  paused: false\n";
    let after = "spec:\n  replicas: 3\n  paused: false\n";
    let held = review("kind/paxone/deployments/api.yaml", before, after);
    app.workspace.reviews.insert(ApprovalId::new("ap-1"), held);
    app
}

fn examined() -> Ui {
    let mut ui = session_ui();
    ui.examine(ApprovalId::new("ap-1"));
    ui
}

#[test]
fn a_cluster_write_waits_on_its_row_with_review_alone() {
    let app = patching();
    let (_, hits) = crate::view(&app, &session_ui(), window(), &mut Fonts::embedded());
    let id = ApprovalId::new("ap-1");
    assert!(
        hits.rect_of(&Target::Approve(id.clone())).is_none(),
        "no approve on the row"
    );
    assert!(hits.rect_of(&Target::Examine(id)).is_some());
}

#[test]
fn a_on_the_rail_opens_the_sheet_of_a_cluster_write_instead_of_approving_it() {
    let app = patching();
    let mut ui = Ui {
        focus: Focus::Rail,
        ..session_ui()
    };
    let acted = press(Key::Char('a'), Modifiers::default(), &mut ui, &app);
    assert!(acted.is_empty(), "nothing approved: {acted:?}");
    assert_eq!(ui.examining(), Some(&ApprovalId::new("ap-1")));
}

fn count(drawn: &[String], word: &str) -> usize {
    drawn.iter().filter(|one| *one == word).count()
}

#[test]
fn the_sheet_draws_the_change_line_by_line_inline_then_split() {
    let app = patching();
    let mut ui = examined();
    let (frame, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let drawn = all_texts(&frame);
    assert_eq!(
        count(&drawn, "replicas"),
        2,
        "removed, then added: {drawn:?}"
    );
    assert_eq!(count(&drawn, "paused"), 1, "an unchanged line once, inline");
    let split = hits
        .rect_of(&Target::SheetView(DiffView::Split))
        .expect("the split view");
    assert!(click(split, &mut ui, &app, &hits).is_empty());
    assert_eq!(ui.sheet().map(|one| one.view), Some(DiffView::Split));
    let (frame, _) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let drawn = all_texts(&frame);
    assert_eq!(
        count(&drawn, "paused"),
        2,
        "an unchanged line stands on both sides"
    );
}

#[test]
fn only_the_sheet_s_approve_button_lets_a_cluster_write_through() {
    let app = patching();
    let mut ui = examined();
    let enter = press(Key::Enter, Modifiers::default(), &mut ui, &app);
    let decided = enter
        .iter()
        .any(|one| matches!(one, Command::Agent(agent::Command::Approve { .. })));
    assert!(!decided, "enter decides nothing: {enter:?}");
    assert!(ui.examining().is_some(), "and the sheet stays");
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let id = ApprovalId::new("ap-1");
    let rect = hits.rect_of(&Target::Approve(id.clone())).expect("approve");
    let acted = click(rect, &mut ui, &app, &hits);
    assert_eq!(acted, [Command::Agent(agent::Command::Approve { id })]);
}
