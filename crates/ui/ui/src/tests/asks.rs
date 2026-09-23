//! The write the agent waits on: what it says, and what the two buttons do.

use groove_controllers::agent_service::Agent;
use groove_gfx::Fonts;
use groove_types::{AgentStatus, ApprovalId, Ask, SessionActivity, SessionId, Timestamp};

use crate::hit::Target;
use crate::tests::{app, click, window};
use crate::{Surface, Ui};

/// The fixture's session, with the writes it waits on.
fn asking(asks: Vec<Ask>) -> groove_controllers::AppState {
    let mut app = app();
    let activity = SessionActivity {
        status: AgentStatus::Idle,
        tool: None,
        asks,
        auto_approve: false,
        changed_at: Timestamp::new(0),
        seen_at: None,
    };
    app.agent.agents.push((
        SessionId::new("a"),
        Agent {
            terminal: None,
            activity,
        },
    ));
    app
}

fn ask(id: &str, op: &str, subject: &str) -> Ask {
    Ask {
        id: ApprovalId::new(id),
        op: op.to_string(),
        subject: subject.to_string(),
    }
}

fn drawn(app: &groove_controllers::AppState, ui: &Ui) -> Vec<String> {
    let (frame, _) = crate::view(app, ui, window(), &mut Fonts::embedded());
    frame.layers()[0]
        .texts
        .iter()
        .map(|run| run.text.clone())
        .collect()
}

fn session_ui() -> Ui {
    Ui {
        surface: Surface::Session,
        ..Ui::default()
    }
}

#[test]
fn the_band_says_the_write_and_offers_the_two_answers() {
    let app = asking(vec![ask("ap-1", "git_commit", "fix: one")]);
    let drawn = drawn(&app, &session_ui());
    assert!(drawn.iter().any(|one| one == "git_commit"), "{drawn:?}");
    assert!(drawn.iter().any(|one| one == "fix: one"), "{drawn:?}");
    assert!(drawn.iter().any(|one| one == "approve"), "{drawn:?}");
    assert!(drawn.iter().any(|one| one == "refuse"), "{drawn:?}");
}

#[test]
fn nothing_stands_there_when_no_write_waits() {
    let app = asking(Vec::new());
    let drawn = drawn(&app, &session_ui());
    assert!(!drawn.iter().any(|one| one == "approve"), "{drawn:?}");
}

#[test]
fn the_band_says_how_many_writes_stand_behind_the_first() {
    let app = asking(vec![
        ask("ap-1", "git_commit", "fix: one"),
        ask("ap-2", "git_push", ""),
    ]);
    let drawn = drawn(&app, &session_ui());
    assert!(drawn.iter().any(|one| one.contains("+1 more")), "{drawn:?}");
    assert!(!drawn.iter().any(|one| one == "git_push"), "one at a time");
}

#[test]
fn approving_asks_the_controller_to_run_it() {
    let app = asking(vec![ask("ap-1", "git_commit", "fix: one")]);
    let mut ui = session_ui();
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let target = Target::Approve(ApprovalId::new("ap-1"));
    let box_ = hits.rect_of(&target).expect("the approve button");
    let acted = click(box_, &mut ui, &app, &hits);
    assert_eq!(
        acted,
        vec![groove_controllers::Command::Agent(
            groove_controllers::agent::Command::Approve {
                id: ApprovalId::new("ap-1")
            }
        )]
    );
}

#[test]
fn refusing_asks_the_controller_to_drop_it() {
    let app = asking(vec![ask("ap-1", "git_commit", "fix: one")]);
    let mut ui = session_ui();
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let target = Target::Refuse(ApprovalId::new("ap-1"));
    let box_ = hits.rect_of(&target).expect("the refuse button");
    let acted = click(box_, &mut ui, &app, &hits);
    assert_eq!(
        acted,
        vec![groove_controllers::Command::Agent(
            groove_controllers::agent::Command::Refuse {
                id: ApprovalId::new("ap-1")
            }
        )]
    );
}
