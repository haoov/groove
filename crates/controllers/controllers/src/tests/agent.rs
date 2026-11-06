use groove_types::{AgentStatus, SessionId};

use crate::agent::Command;
use crate::dispatch;
use crate::tests::fixture::{self, until};

fn agent_cmd(command: Command) -> crate::Command {
    crate::Command::Agent(command)
}

#[test]
fn start_say_and_exit_travel_through_the_loop() {
    let (home, spawner, services, mut state) = fixture::with_explorer();
    let id = SessionId::new("explorer-1");
    let start = Command::Start {
        session: id.clone(),
    };
    dispatch(agent_cmd(start), &mut state, &services, &spawner);
    until(&spawner, &services, &mut state, |s| {
        s.agent.stream(&id).is_some()
    });
    assert_eq!(state.agent.activity(&id).unwrap().status, AgentStatus::Idle);
    let say = |text: &str| Command::Say {
        session: id.clone(),
        text: text.into(),
    };
    dispatch(agent_cmd(say("hi")), &mut state, &services, &spawner);
    dispatch(agent_cmd(say("exit")), &mut state, &services, &spawner);
    until(&spawner, &services, &mut state, |s| {
        matches!(
            s.agent.activity(&id).unwrap().status,
            AgentStatus::Exited { .. }
        )
    });
    let agent = state.agent.agent(&id).unwrap();
    assert_eq!(agent.activity.status, AgentStatus::Exited { code: 7 });
    let (_, said) = fixture::heard(home.path(), "explorer-1").expect("a run");
    assert_eq!(said, ["hi", "exit"]);
    assert!(
        home.path()
            .join("data/agent-launch/explorer-1.prompt.md")
            .is_file()
    );
}

#[test]
fn end_forgets_the_agent() {
    let (_home, spawner, services, mut state) = fixture::with_explorer();
    let id = SessionId::new("explorer-1");
    let start = Command::Start {
        session: id.clone(),
    };
    dispatch(agent_cmd(start), &mut state, &services, &spawner);
    until(&spawner, &services, &mut state, |s| {
        s.agent.agent(&id).is_some()
    });
    let end = Command::End {
        session: id.clone(),
    };
    dispatch(agent_cmd(end), &mut state, &services, &spawner);
    assert!(state.agent.agent(&id).is_none());
}

#[test]
fn a_start_for_an_unknown_session_does_nothing() {
    let (_home, spawner, services, mut state) = fixture::with_explorer();
    let id = SessionId::new("nope");
    dispatch(
        agent_cmd(Command::Start {
            session: id.clone(),
        }),
        &mut state,
        &services,
        &spawner,
    );
    spawner.drain(&mut state, &services);
    assert!(state.agent.agent(&id).is_none());
}
