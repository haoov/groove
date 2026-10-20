use groove_types::{AgentStatus, SessionId};

use crate::agent::Command;
use crate::dispatch;
use crate::tests::fixture::{self, until};

fn agent_cmd(command: Command) -> crate::Command {
    crate::Command::Agent(command)
}

#[test]
fn start_send_and_exit_travel_through_the_loop() {
    let (home, spawner, services, mut state) = fixture::with_explorer();
    let id = SessionId::new("explorer-1");

    dispatch(
        agent_cmd(Command::Start {
            session: id.clone(),
            cols: 40,
            rows: 6,
        }),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| {
        s.agent.agent(&id).is_some()
    });
    let agent = state.agent.agent(&id).unwrap();
    assert_eq!(agent.activity.status, AgentStatus::Idle);
    assert_eq!(agent.terminal.as_ref().unwrap().size(), (40, 6));

    until(&spawner, &services, &mut state, |s| {
        s.agent
            .agent(&id)
            .unwrap()
            .terminal
            .as_ref()
            .unwrap()
            .screen()
            .line(0)
            .starts_with("ready")
    });
    dispatch(
        agent_cmd(Command::Send {
            session: id.clone(),
            bytes: b"hi\n".to_vec(),
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

    let agent = state.agent.agent(&id).unwrap();
    assert_eq!(agent.activity.status, AgentStatus::Exited { code: 7 });
    let screen = agent.terminal.as_ref().unwrap().screen();
    assert_eq!(screen.line(0), "ready --session-id");
    assert_eq!(screen.line(2), "got hi");
    assert!(
        home.path()
            .join("data/agent-launch/explorer-1.prompt.md")
            .is_file()
    );
}

#[test]
fn resize_reaches_the_terminal_and_end_forgets_it() {
    let (_home, spawner, services, mut state) = fixture::with_explorer();
    let id = SessionId::new("explorer-1");
    dispatch(
        agent_cmd(Command::Start {
            session: id.clone(),
            cols: 40,
            rows: 6,
        }),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| {
        s.agent.agent(&id).is_some()
    });
    dispatch(
        agent_cmd(Command::Resize {
            session: id.clone(),
            cols: 30,
            rows: 8,
        }),
        &mut state,
        &services,
        &spawner,
    );
    assert_eq!(
        state
            .agent
            .agent(&id)
            .unwrap()
            .terminal
            .as_ref()
            .unwrap()
            .size(),
        (30, 8)
    );
    dispatch(
        agent_cmd(Command::End {
            session: id.clone(),
        }),
        &mut state,
        &services,
        &spawner,
    );
    assert!(state.agent.agent(&id).is_none());
}

#[test]
fn a_start_for_an_unknown_session_does_nothing() {
    let (_home, spawner, services, mut state) = fixture::with_explorer();
    let id = SessionId::new("nope");
    dispatch(
        agent_cmd(Command::Start {
            session: id.clone(),
            cols: 40,
            rows: 6,
        }),
        &mut state,
        &services,
        &spawner,
    );
    spawner.drain(&mut state, &services);
    assert!(state.agent.agent(&id).is_none());
}
