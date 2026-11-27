//! The fixture's agent: its session, the writes it waits on, and a machine set up.

use groove_controllers::agent_service::Agent;
use groove_types::{AgentStatus, ApprovalId, Ask, SessionActivity, SessionId, Timestamp};

use crate::tests::app;
use crate::{Surface, Ui};

/// The fixture's session, with the writes it waits on.
pub(super) fn asking(asks: Vec<Ask>) -> groove_controllers::AppState {
    let mut app = app();
    let activity = SessionActivity {
        status: AgentStatus::Idle,
        tool: None,
        asks,
        changed_at: Timestamp::new(0),
        seen_at: None,
    };

    app.agent.agents.push((
        SessionId::new("a"),
        Agent {
            stream: None,
            permissions: Vec::new(),
            commands: Vec::new(),
            limits: Default::default(),
            windows: Vec::new(),
            chosen: None,
            activity,
            started_at: Timestamp::new(0),
            launch: 0,
        },
    ));
    app
}

/// The fixture's agent running headless, its conversation drawn.
pub(super) fn headless(app: &mut groove_controllers::AppState) {
    app.agent.agents[0].1.stream = Some(groove_controllers::agent_service::Stream::idle());
}

pub(super) fn ask(id: &str, op: &str, subject: &str) -> Ask {
    Ask {
        id: ApprovalId::new(id),
        op: op.to_string(),
        subject: subject.to_string(),
        text: subject.to_string(),
        worktree: None,
        reviewed: false,
    }
}

pub(super) fn session_ui() -> Ui {
    Ui {
        surface: Surface::Session,
        ..Ui::default()
    }
}

/// The config of a machine whose task sources are set up.
pub(super) fn sourced(
    mut app: groove_controllers::AppState,
    github: bool,
    notion: bool,
) -> groove_controllers::AppState {
    let git = r#"{ "git": { "worktree_root": "code/worktrees" } }"#;
    let mut config: groove_types::Config = serde_json::from_str(git).unwrap();
    config.notion = notion.then(|| groove_types::NotionConfig {
        token: "t".into(),
        database_id: "d".into(),
        user_id: "u".into(),
        assignee: Some("Assignee".into()),
        sprint: Some("Sprint".into()),
        sprint_status: None,
        properties: Default::default(),
        status_map: Default::default(),
        priority_map: Default::default(),
        filters: groove_types::FilterConfig {
            exclude_statuses: Vec::new(),
        },
        task_template_page_id: None,
        default_project_id: None,
    });
    config.github = github.then(|| groove_types::GithubConfig {
        host: "github.com".into(),
        token: None,
        properties: Default::default(),
        status_map: Default::default(),
        priority_map: Default::default(),
    });
    app.config.config = Some(config);
    app
}

/// One of Groove's own skills, `groove:<name>`.
pub(super) fn skill(name: &str) -> groove_types::Skill {
    groove_types::Skill {
        id: format!("groove:{name}"),
        plugin: "groove".into(),
        name: name.into(),
        description: "What it does.".into(),
        hint: String::new(),
        label: name.into(),
        kinds: Vec::new(),
        resources: Vec::new(),
        editable: false,
        enabled: true,
        changed_at: Timestamp::new(0),
    }
}
