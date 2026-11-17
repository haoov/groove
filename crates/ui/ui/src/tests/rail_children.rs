//! The rail keeps each child right under its parent, whatever the child's agent does.

use groove_types::{AgentStatus, ExternalId, SessionActivity, SessionId, SessionKind, Timestamp};

use groove_controllers::agent_service::Agent;

use crate::Ui;
use crate::tests::{app, open};

#[test]
fn a_working_child_stands_under_its_idle_parent() {
    let mut app = app();
    let mut parent = open("parent", "Maintenance 2026 week 47");
    parent.session.kind = SessionKind::Task {
        external_id: ExternalId::new("notion:MAINT-47"),
    };
    parent.state.maintenance = true;
    let mut child = open("child", "VeleroBackupFailed");
    child.state.parent = Some(SessionId::new("parent"));
    app.session.open.insert(0, child);
    app.session.open.push(parent);
    let activity = SessionActivity {
        status: AgentStatus::Working,
        tool: None,
        asks: Vec::new(),
        changed_at: Timestamp::new(0),
        seen_at: None,
    };
    let agent = Agent {
        terminal: None,
        activity,
        started_at: Timestamp::new(0),
        launch: 0,
    };
    app.agent.agents.push((SessionId::new("child"), agent));

    let order: Vec<String> = crate::views::rail::shown(&app, &Ui::default())
        .into_iter()
        .map(|one| one.session.id.to_string())
        .collect();
    let at = |id: &str| order.iter().position(|one| one == id).expect(id);
    assert_eq!(at("child"), at("parent") + 1, "{order:?}");
}
