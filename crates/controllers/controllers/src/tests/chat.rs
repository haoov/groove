//! The agent's conversation: followed from the transcript its hooks name, answered from Groove's prompt.

use std::time::Duration;

use groove_agent_service::Post;
use groove_types::chat::Entry;
use groove_types::{HookKind, SessionId};

use crate::agent::{Command, hooked};
use crate::tests::fixture::{self, until};
use crate::{AppState, Services, SyncSpawner, dispatch};

/// The explorer's agent, started and showing its first line.
fn started(state: &mut AppState, services: &Services, spawner: &SyncSpawner) -> SessionId {
    let id = SessionId::new("explorer-1");
    let start = Command::Start {
        session: id.clone(),
        cols: 40,
        rows: 6,
    };
    dispatch(crate::Command::Agent(start), state, services, spawner);
    until(spawner, services, state, |s| {
        line(s, &id, 0).starts_with("ready")
    });
    id
}

fn line(state: &AppState, id: &SessionId, row: usize) -> String {
    let terminal = state.agent.terminal(id);
    terminal
        .map(|one| one.screen().line(row))
        .unwrap_or_default()
}

#[test]
fn a_hook_naming_a_transcript_draws_what_it_holds() {
    let (home, spawner, services, mut state) = fixture::with_explorer();
    let id = started(&mut state, &services, &spawner);
    let path = home.path().join("session.jsonl");
    let record = r#"{"type":"user","message":{"content":"cache the quotes"}}"#;
    std::fs::write(&path, format!("{record}\n")).unwrap();
    let post = Post {
        session: id.clone(),
        kind: HookKind::UserPromptSubmit,
        tool: None,
        transcript: Some(path.clone()),
    };
    hooked(&mut state, &spawner, post);
    spawner.settle(Duration::from_millis(300), &mut state, &services);
    let chat = state.agent.chat(&id).unwrap();
    assert_eq!(chat.entries(), [Entry::You("cache the quotes".into())]);
    assert_eq!(state.agent.transcript(&id), Some(path.as_path()));
}

#[test]
fn a_message_sent_from_groove_s_prompt_reaches_the_agent_and_waits_in_the_conversation() {
    let (home, spawner, services, mut state) = fixture::with_explorer();
    let id = started(&mut state, &services, &spawner);
    state.agent.follow(&id, home.path().join("session.jsonl"));
    let say = Command::Say {
        session: id.clone(),
        text: "  hello  ".into(),
    };
    dispatch(crate::Command::Agent(say), &mut state, &services, &spawner);
    until(&spawner, &services, &mut state, |s| {
        line(s, &id, 2) == "got hello"
    });
    let chat = state.agent.chat(&id).unwrap();
    assert_eq!(
        chat.queued(),
        ["hello"],
        "shown before the transcript says it"
    );
}
