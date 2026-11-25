//! A session waiting on a cluster write: the Diff tab holds the ask on its band, then the change, until it is answered.

use groove_controllers::workspace_service::prepare;
use groove_controllers::{AppState, Command, agent};
use groove_gfx::Fonts;
use groove_types::ApprovalId;

use super::screen::{ask, asking, session_ui};
use crate::hit::Target;
use crate::input::{Key, Modifiers};
use crate::tests::{all_texts, click, press, window};
use crate::views::session::Tab;
use crate::{Focus, Ui};

fn pairs() -> Vec<(String, String, String)> {
    let before = "spec:\n  replicas: 1\n  paused: false\n".to_string();
    let after = "spec:\n  replicas: 3\n  paused: false\n".to_string();
    vec![("kind/paxone/Deployment/api.yaml".into(), before, after)]
}

/// A patch waiting on the selected session, its change shown in place of the worktree's.
fn patching() -> AppState {
    let mut one = ask("ap-1", "kube_patch", "kind · paxone/deployments/api");
    one.reviewed = true;
    let mut app = asking(vec![one]);
    app.workspace
        .review(&ApprovalId::new("ap-1"), prepare(pairs()));
    app
}

fn settled(app: &AppState, mut ui: Ui) -> Ui {
    ui.settle(app);
    ui
}

#[test]
fn the_diff_tab_holds_the_ask_then_the_change_whatever_tab_was_asked() {
    let app = patching();
    let mut ui = settled(&app, session_ui());
    assert_eq!(ui.session.tab, Tab::Diff);
    let (frame, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let drawn = all_texts(&frame);
    let count = |word: &str| drawn.iter().filter(|one| *one == word).count();
    assert!(count("replicas") >= 2, "removed, then added: {drawn:?}");
    let overview = hits.rect_of(&Target::Tab(Tab::Overview)).expect("the tab");
    click(overview, &mut ui, &app, &hits);
    ui.settle(&app);
    assert_eq!(ui.session.tab, Tab::Diff, "the ask is answered first");
}

#[test]
fn a_on_the_rail_neither_approves_nor_opens_a_sheet_over_the_ask() {
    let app = patching();
    let mut ui = settled(
        &app,
        Ui {
            focus: Focus::Rail,
            ..session_ui()
        },
    );
    let acted = press(Key::Char('a'), Modifiers::default(), &mut ui, &app);
    assert!(acted.is_empty(), "nothing approved: {acted:?}");
    ui.settle(&app);
    assert_eq!(ui.examining(), None, "the Diff tab holds the ask");
}

#[test]
fn only_the_band_s_approve_button_lets_a_cluster_write_through() {
    let app = patching();
    let mut ui = settled(&app, session_ui());
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let id = ApprovalId::new("ap-1");
    let rect = hits.rect_of(&Target::Approve(id.clone())).expect("approve");
    let acted = click(rect, &mut ui, &app, &hits);
    assert_eq!(acted, [Command::Agent(agent::Command::Approve { id })]);
}

#[test]
fn once_the_write_is_answered_the_session_goes_back_to_the_tab_it_stood_on() {
    let app = patching();
    let mut ui = session_ui();
    ui.session.tab = Tab::Overview;
    ui.settle(&app);
    assert_eq!(ui.session.tab, Tab::Diff);
    let mut answered = app;
    answered.agent = asking(Vec::new()).agent;
    ui.settle(&answered);
    assert_eq!(ui.session.tab, Tab::Overview);
}

#[test]
fn a_write_is_answered_only_once_its_change_is_shown() {
    let mut one = ask("ap-1", "kube_exec", "kind · paxone/pods/db-0");
    one.reviewed = true;
    let app = asking(vec![one]);
    let ui = settled(&app, session_ui());
    let (frame, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let id = ApprovalId::new("ap-1");
    assert!(hits.rect_of(&Target::Approve(id.clone())).is_none());
    assert!(hits.rect_of(&Target::Refuse(id)).is_none());
    let drawn = all_texts(&frame);
    assert!(
        drawn.iter().any(|one| one == "Reading the change…"),
        "{drawn:?}"
    );
}
