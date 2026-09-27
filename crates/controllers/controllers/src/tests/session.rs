use groove_types::{AgentStatus, SessionKind};

use crate::session::Command;
use crate::tests::fixture::{self, services, state, until};
use crate::{AppState, Command as Cmd, Services, SyncSpawner, dispatch};

#[test]
fn opening_an_explorer_adds_a_row_selects_it_and_starts_its_agent() {
    let (_home, spawner, services, mut state) = crate::tests::fixture::fresh();
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
    let (_home, spawner, services, mut state) = crate::tests::fixture::fresh();
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
fn an_explorer_has_its_own_directory_before_it_holds_any_repo() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    dispatch(
        Cmd::Session(Command::OpenExplorer { title: None }),
        &mut state,
        &services,
        &spawner,
    );
    let id = state.session.selected.clone().unwrap();
    let dir = home.path().join("code/worktrees").join(id.as_str());
    until(&spawner, &services, &mut state, |_| dir.is_dir());
}

#[test]
fn the_agent_runs_at_the_worktree_root_never_in_a_session_directory() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
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
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    for title in ["keep", "drop", "closed"] {
        open_explorer(&mut state, &services, &spawner, title);
    }
    let ids: Vec<_> = state
        .session
        .open
        .iter()
        .map(|o| o.session.id.clone())
        .collect();
    until(&spawner, &services, &mut state, |s| {
        ids.iter().all(|id| s.agent.agent(id).is_some())
    });

    let acts = [
        Command::RenameExplorer {
            session: ids[0].clone(),
            title: "kept".into(),
        },
        Command::Delete {
            session: ids[1].clone(),
        },
        Command::Close {
            session: ids[2].clone(),
        },
        Command::Select {
            session: ids[0].clone(),
        },
    ];
    for act in acts {
        dispatch(Cmd::Session(act), &mut state, &services, &spawner);
    }
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

pub(super) fn open_explorer(
    state: &mut AppState,
    services: &Services,
    spawner: &SyncSpawner,
    title: &str,
) {
    dispatch(
        Cmd::Session(Command::OpenExplorer {
            title: Some(title.into()),
        }),
        state,
        services,
        spawner,
    );
}

#[test]
fn a_closed_session_stays_on_the_board_and_comes_back_when_picked() {
    let home = tempfile::tempdir().unwrap();
    crate::tests::fixture::pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    let dir = crate::tests::fixture::worktree(&mut state, &services, &spawner);
    assert!(!dir.is_empty(), "the session has a worktree");
    let id = state.session.selected.clone().expect("a session");

    dispatch(
        Cmd::Session(Command::Close {
            session: id.clone(),
        }),
        &mut state,
        &services,
        &spawner,
    );
    spawner.drain(&mut state, &services);
    assert!(state.session.open.is_empty(), "the rail lets it go");

    dispatch(Cmd::Session(Command::List), &mut state, &services, &spawner);
    until(&spawner, &services, &mut state, |s| {
        !s.session.living.is_empty()
    });
    let living = state.session.living.first().expect("the board keeps it");
    assert_eq!(living.session.id, id);
    assert_eq!(living.worktrees.len(), 1, "its worktree is still on disk");
    assert_eq!(living.repos, 1, "and so is its repo");

    dispatch(
        Cmd::Session(Command::Open {
            session: id.clone(),
        }),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| {
        s.agent.agent(&id).is_some()
    });
    assert_eq!(state.session.open.len(), 1, "picking it brings it back");
    assert_eq!(state.session.selected.as_ref(), Some(&id));
    assert!(state.errors.is_empty(), "{:?}", state.errors);
}

#[test]
fn an_explorer_with_no_worktree_is_still_on_the_board() {
    let (_home, spawner, services, mut state) = crate::tests::fixture::fresh();
    open_explorer(&mut state, &services, &spawner, "bare");
    let id = state.session.selected.clone().expect("a session");
    dispatch(
        Cmd::Session(Command::Close {
            session: id.clone(),
        }),
        &mut state,
        &services,
        &spawner,
    );
    spawner.drain(&mut state, &services);

    dispatch(Cmd::Session(Command::List), &mut state, &services, &spawner);
    until(&spawner, &services, &mut state, |s| {
        !s.session.living.is_empty()
    });
    let living = state.session.living.first().expect("it is on the board");
    assert_eq!(living.session.id, id);
    assert!(living.worktrees.is_empty(), "with nothing under it");
}

#[test]
fn the_board_holds_every_session_without_being_asked() {
    let (_home, spawner, services, mut state) = crate::tests::fixture::fresh();
    open_explorer(&mut state, &services, &spawner, "one");
    until(&spawner, &services, &mut state, |s| {
        !s.session.living.is_empty()
    });
    assert_eq!(state.session.living.len(), 1, "opening one lists it");

    open_explorer(&mut state, &services, &spawner, "two");
    until(&spawner, &services, &mut state, |s| {
        s.session.living.len() == 2
    });
    let id = state.session.selected.clone().expect("the second");
    dispatch(
        Cmd::Session(Command::Delete { session: id }),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| {
        s.session.living.len() == 1
    });
    assert!(state.errors.is_empty(), "{:?}", state.errors);
}

#[test]
fn a_session_taken_away_leaves_the_board_at_once() {
    let (_home, spawner, services, mut state) = crate::tests::fixture::fresh();
    dispatch(
        Cmd::Session(Command::OpenExplorer { title: None }),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| {
        !s.session.living.is_empty()
    });
    let session = state.session.open[0].session.id.clone();
    assert_eq!(state.session.living.len(), 1, "the board holds it");

    dispatch(
        Cmd::Session(Command::Delete {
            session: session.clone(),
        }),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| {
        s.session.living.is_empty()
    });
    assert!(
        state.session.living.is_empty(),
        "and lets it go without being asked again"
    );
}
