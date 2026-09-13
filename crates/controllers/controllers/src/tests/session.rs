use groove_types::{AgentStatus, SessionKind};

use crate::session::Command;
use crate::tests::fixture::{state, until};
use crate::{Command as Cmd, SyncSpawner, dispatch};

#[test]
fn opening_an_explorer_adds_a_row_selects_it_and_starts_its_agent() {
    let home = tempfile::tempdir().unwrap();
    let spawner = SyncSpawner::new().unwrap();
    let mut state = state(home.path());
    dispatch(
        Cmd::Session(Command::OpenExplorer {
            title: Some(" my idea ".into()),
        }),
        &mut state,
        &spawner,
    );
    assert_eq!(state.session.open.len(), 1);
    let open = state.session.selected().unwrap();
    assert_eq!(open.session.title, "my idea");
    assert_eq!(open.session.kind, SessionKind::Explorer);
    assert!(open.session.id.as_str().starts_with("explorer-"));
    assert!(open.state.opened_at.is_some());
    let id = open.session.id.clone();

    until(&spawner, &mut state, |s| s.agent.agent(&id).is_some());
    assert_eq!(state.agent.activity(&id).unwrap().status, AgentStatus::Idle);
    assert_eq!(
        state
            .agent
            .agent(&id)
            .unwrap()
            .terminal
            .as_ref()
            .unwrap()
            .size(),
        (80, 24)
    );
}

#[test]
fn closing_ends_the_agent_and_moves_the_selection() {
    let home = tempfile::tempdir().unwrap();
    let spawner = SyncSpawner::new().unwrap();
    let mut state = state(home.path());
    for _ in 0..3 {
        dispatch(
            Cmd::Session(Command::OpenExplorer { title: None }),
            &mut state,
            &spawner,
        );
    }
    assert!(
        state
            .session
            .open
            .iter()
            .all(|o| o.session.title == "Explorer")
    );
    let ids: Vec<_> = state
        .session
        .open
        .iter()
        .map(|o| o.session.id.clone())
        .collect();
    until(&spawner, &mut state, |s| {
        ids.iter().all(|id| s.agent.agent(id).is_some())
    });
    assert_eq!(state.session.selected.as_ref(), Some(&ids[2]));

    dispatch(
        Cmd::Session(Command::Select {
            session: ids[1].clone(),
        }),
        &mut state,
        &spawner,
    );
    assert_eq!(state.session.selected.as_ref(), Some(&ids[1]));
    assert!(state.session.get(&ids[1]).unwrap().state.seen_at.is_some());

    dispatch(
        Cmd::Session(Command::Close {
            session: ids[1].clone(),
        }),
        &mut state,
        &spawner,
    );
    assert_eq!(state.session.open.len(), 2);
    assert!(state.agent.agent(&ids[1]).is_none());
    assert_eq!(
        state.session.selected.as_ref(),
        Some(&ids[2]),
        "the row that took its place"
    );

    dispatch(
        Cmd::Session(Command::Close {
            session: ids[2].clone(),
        }),
        &mut state,
        &spawner,
    );
    assert_eq!(
        state.session.selected.as_ref(),
        Some(&ids[0]),
        "the last row"
    );
    dispatch(
        Cmd::Session(Command::Close {
            session: ids[0].clone(),
        }),
        &mut state,
        &spawner,
    );
    assert!(state.session.selected.is_none());
    assert!(state.agent.agents.is_empty());
}

#[test]
fn the_agent_runs_at_the_worktree_root_never_in_a_session_directory() {
    let home = tempfile::tempdir().unwrap();
    let spawner = SyncSpawner::new().unwrap();
    let mut state = state(home.path());
    std::fs::create_dir_all(home.path().join("wt")).unwrap();
    let mut config: groove_types::Config =
        serde_json::from_str(r#"{ "git": { "worktree_root": "~/wt" } }"#).unwrap();
    config.ui.theme = groove_types::ThemeName::Mocha;
    state.config.config = Some(config);
    assert_eq!(
        state.config.worktree_root(home.path()),
        home.path().join("wt")
    );

    dispatch(
        Cmd::Session(Command::OpenExplorer { title: None }),
        &mut state,
        &spawner,
    );
    let id = state.session.selected.clone().unwrap();
    until(&spawner, &mut state, |s| {
        s.agent
            .agent(&id)
            .and_then(|a| a.terminal.as_ref())
            .is_some_and(|t| t.screen().line(0).starts_with("ready"))
    });
    dispatch(
        Cmd::Agent(crate::agent::Command::Send {
            session: id.clone(),
            bytes: b"pwd\n".to_vec(),
        }),
        &mut state,
        &spawner,
    );
    until(&spawner, &mut state, |s| {
        matches!(
            s.agent.activity(&id).unwrap().status,
            AgentStatus::Exited { .. }
        )
    });
    let screen = state
        .agent
        .agent(&id)
        .unwrap()
        .terminal
        .as_ref()
        .unwrap()
        .screen();
    assert_eq!(
        screen.line(2),
        format!("got {}", home.path().join("wt").display())
    );
}
