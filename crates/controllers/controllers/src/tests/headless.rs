//! A headless agent: its words, its asks and their answers, and its interrupt go over its stream.

use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::time::{Duration, Instant};

use groove_types::{AgentStatus, SessionId};

use crate::agent::Command;
use crate::tests::fixture;
use crate::{AppState, Services, SyncSpawner, dispatch};

/// Reads a message, asks for Bash, settles once let, then ends on the next line, an interrupt.
const FAKE_STREAM: &str = r#"#!/bin/sh
case " $* " in *" --input-format stream-json "*) ;; *) exit 9;; esac
read said
case "$said" in *'"content":"hello"'*) ;; *) exit 8;; esac
printf '%s\n' '{"type":"control_request","request_id":"r1","request":{"subtype":"can_use_tool","tool_name":"Bash","input":{"command":"cargo test"}}}'
read answer
case "$answer" in *'"behavior":"allow"'*) printf '%s\n' '{"type":"result","subtype":"success"}';; esac
read stop
case "$stop" in *'"subtype":"interrupt"'*) exit 7;; esac
exit 6
"#;

fn headless(home: &Path, state: &mut AppState) {
    let claude = home.join(".local/bin/claude");
    std::fs::write(&claude, FAKE_STREAM).unwrap();
    std::fs::set_permissions(&claude, std::fs::Permissions::from_mode(0o755)).unwrap();
    let config = state.config.config.as_mut().expect("a config");
    config.preferences.conversation = true;
}

fn run(command: Command, state: &mut AppState, services: &Services, spawner: &SyncSpawner) {
    dispatch(crate::Command::Agent(command), state, services, spawner);
}

/// Waits for `done`, the runtime let run between looks: the stream's pipes are read on it.
#[track_caller]
fn until(
    spawner: &SyncSpawner,
    services: &Services,
    state: &mut AppState,
    done: impl Fn(&AppState) -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while !done(state) {
        assert!(
            Instant::now() < deadline,
            "timed out; it failed: {:?}",
            state.errors
        );
        spawner.settle(Duration::from_millis(10), state, services);
    }
}

fn status(state: &AppState, id: &SessionId) -> AgentStatus {
    state.agent.activity(id).expect("an agent").status.clone()
}

#[test]
fn a_headless_agent_hears_a_message_asks_leave_settles_and_stops_on_an_interrupt() {
    let (home, spawner, services, mut state) = fixture::with_explorer();
    headless(home.path(), &mut state);
    let (id, services) = (SessionId::new("explorer-1"), &services);
    let start = Command::Start {
        session: id.clone(),
        cols: 40,
        rows: 6,
    };
    run(start, &mut state, services, &spawner);
    until(&spawner, services, &mut state, |s| {
        s.agent.stream(&id).is_some()
    });
    assert!(state.agent.terminal(&id).is_none());
    let say = Command::Say {
        session: id.clone(),
        text: "hello".into(),
    };
    run(say, &mut state, services, &spawner);
    until(&spawner, services, &mut state, |s| {
        !s.agent.permissions(&id).is_empty()
    });
    let asked = &state.agent.permissions(&id)[0];
    assert_eq!(
        (asked.tool.as_str(), asked.detail.as_deref()),
        ("Bash", Some("cargo test"))
    );
    assert_eq!(status(&state, &id), AgentStatus::Asking);
    let answer = Command::Answer {
        session: id.clone(),
        id: "r1".into(),
        allow: true,
        picked: Vec::new(),
    };
    run(answer, &mut state, services, &spawner);
    until(&spawner, services, &mut state, |s| {
        status(s, &id) == AgentStatus::Idle
    });
    assert!(state.agent.permissions(&id).is_empty());
    run(
        Command::Interrupt {
            session: id.clone(),
        },
        &mut state,
        services,
        &spawner,
    );
    until(&spawner, services, &mut state, |s| {
        status(s, &id) == AgentStatus::Exited { code: 7 }
    });
}
