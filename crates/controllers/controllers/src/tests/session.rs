use groove_types::{AgentStatus, SessionKind};

use crate::session::Command;
use crate::tests::fixture::{self, services, state, until};
use crate::{Command as Cmd, SyncSpawner, dispatch};

#[test]
fn opening_an_explorer_adds_a_row_selects_it_and_starts_its_agent() {
    let home = tempfile::tempdir().unwrap();
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner);
    let mut state = state(home.path());
    dispatch(
        Cmd::Session(Command::OpenExplorer {
            title: Some(" my idea ".into()),
        }),
        &mut state,
        &services,
        &spawner,
    );
    assert_eq!(state.session.open.len(), 1);
    let open = state.session.selected().unwrap();
    assert_eq!(open.session.title, "my idea");
    assert_eq!(open.session.kind, SessionKind::Explorer);
    assert!(open.session.id.as_str().starts_with("explorer-"));
    assert!(open.state.opened_at.is_some());
    let id = open.session.id.clone();

    until(&spawner, &services, &mut state, |s| {
        s.agent.agent(&id).is_some()
    });
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
    let services = services(&spawner);
    let mut state = state(home.path());
    for _ in 0..3 {
        dispatch(
            Cmd::Session(Command::OpenExplorer { title: None }),
            &mut state,
            &services,
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
    until(&spawner, &services, &mut state, |s| {
        ids.iter().all(|id| s.agent.agent(id).is_some())
    });
    assert_eq!(state.session.selected.as_ref(), Some(&ids[2]));

    dispatch(
        Cmd::Session(Command::Select {
            session: ids[1].clone(),
        }),
        &mut state,
        &services,
        &spawner,
    );
    assert_eq!(state.session.selected.as_ref(), Some(&ids[1]));
    assert!(state.session.get(&ids[1]).unwrap().state.seen_at.is_some());

    dispatch(
        Cmd::Session(Command::Close {
            session: ids[1].clone(),
        }),
        &mut state,
        &services,
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
        &services,
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
        &services,
        &spawner,
    );
    assert!(state.session.selected.is_none());
    assert!(state.agent.agents.is_empty());
}

#[test]
fn the_agent_runs_at_the_worktree_root_never_in_a_session_directory() {
    let home = tempfile::tempdir().unwrap();
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner);
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
        &services,
        &spawner,
    );
    let id = state.session.selected.clone().unwrap();
    until(&spawner, &services, &mut state, |s| {
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
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| {
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

#[test]
fn explorers_persist_and_the_rail_restores_with_agents() {
    let home = tempfile::tempdir().unwrap();
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner);
    let mut state = state(home.path());
    dispatch(
        Cmd::Session(Command::OpenExplorer {
            title: Some("keep".into()),
        }),
        &mut state,
        &services,
        &spawner,
    );
    dispatch(
        Cmd::Session(Command::OpenExplorer {
            title: Some("drop".into()),
        }),
        &mut state,
        &services,
        &spawner,
    );
    dispatch(
        Cmd::Session(Command::OpenExplorer {
            title: Some("closed".into()),
        }),
        &mut state,
        &services,
        &spawner,
    );
    let ids: Vec<_> = state
        .session
        .open
        .iter()
        .map(|o| o.session.id.clone())
        .collect();
    until(&spawner, &services, &mut state, |s| {
        ids.iter().all(|id| s.agent.agent(id).is_some())
    });

    dispatch(
        Cmd::Session(Command::RenameExplorer {
            session: ids[0].clone(),
            title: "kept".into(),
        }),
        &mut state,
        &services,
        &spawner,
    );
    dispatch(
        Cmd::Session(Command::DiscardExplorer {
            session: ids[1].clone(),
        }),
        &mut state,
        &services,
        &spawner,
    );
    dispatch(
        Cmd::Session(Command::Close {
            session: ids[2].clone(),
        }),
        &mut state,
        &services,
        &spawner,
    );
    dispatch(
        Cmd::Session(Command::Select {
            session: ids[0].clone(),
        }),
        &mut state,
        &services,
        &spawner,
    );
    spawner.drain(&mut state, &services);
    assert!(state.errors.is_empty(), "{:?}", state.errors);

    let mut fresh = fixture::state(home.path());
    dispatch(
        Cmd::Session(Command::Restore),
        &mut fresh,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut fresh, |s| {
        s.agent.agent(&ids[0]).is_some()
    });
    let titles: Vec<&str> = fresh
        .session
        .open
        .iter()
        .map(|o| o.session.title.as_str())
        .collect();
    assert_eq!(
        titles,
        ["kept"],
        "discarded and closed rows stay off the rail"
    );
    assert_eq!(fresh.session.selected.as_ref(), Some(&ids[0]));
    assert!(
        fresh
            .session
            .get(&ids[0])
            .unwrap()
            .state
            .opened_at
            .is_some()
    );
    assert!(fresh.errors.is_empty(), "{:?}", fresh.errors);
}
