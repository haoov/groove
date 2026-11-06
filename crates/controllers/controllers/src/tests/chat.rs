//! The agent's conversation: followed from the transcript its hooks name, answered from Groove's prompt.

use std::time::Duration;

use groove_agent_service::Post;
use groove_types::chat::Entry;
use groove_types::{HookKind, SessionId};

use crate::agent::{Command, hooked};
use crate::tests::fixture::{self, until};
use crate::{AppState, Services, SyncSpawner, dispatch};

/// The explorer's agent, started on its stream.
fn started(state: &mut AppState, services: &Services, spawner: &SyncSpawner) -> SessionId {
    let id = SessionId::new("explorer-1");
    let start = Command::Start {
        session: id.clone(),
    };
    dispatch(crate::Command::Agent(start), state, services, spawner);
    until(spawner, services, state, |s| s.agent.stream(&id).is_some());
    id
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
    let heard = || fixture::heard(home.path(), "explorer-1").map(|(_, said)| said);
    until(&spawner, &services, &mut state, |_| {
        heard() == Some(vec!["hello".into()])
    });
    let chat = state.agent.chat(&id).unwrap();
    assert_eq!(
        chat.sent(),
        ["hello"],
        "shown before the transcript says it"
    );
}
