//! A session's conversation, heard from the transcript it follows.

use std::path::PathBuf;

use groove_types::chat::{Entry, Heard};
use groove_types::{AgentStatus, SessionId, Timestamp};

use crate::{Agent, Event, State, apply};

fn state() -> State {
    let mut state = State::default();
    let agent = Agent {
        terminal: None,
        activity: crate::activity(AgentStatus::Idle, Timestamp::new(100)),
        started_at: Timestamp::new(100),
        launch: 0,
    };
    state.agents.push((SessionId::new("s"), agent));
    state
}

fn heard(state: &mut State, transcript: &str, said: &str) {
    let event = Event::Heard {
        session: SessionId::new("s"),
        transcript: PathBuf::from(transcript),
        heard: vec![Heard::You(said.into())],
    };
    apply(state, event);
}

#[test]
fn what_the_followed_transcript_says_becomes_the_conversation() {
    let (mut state, session) = (state(), SessionId::new("s"));
    assert!(
        state
            .follow(&session, PathBuf::from("/t/a.jsonl"))
            .is_some()
    );
    heard(&mut state, "/t/a.jsonl", "hi");
    let chat = state.chat(&session).expect("a conversation");
    assert_eq!(chat.entries(), [Entry::You("hi".into())]);
}

#[test]
fn a_new_transcript_starts_the_conversation_again_and_the_old_one_is_not_heard() {
    let (mut state, session) = (state(), SessionId::new("s"));
    state.follow(&session, PathBuf::from("/t/a.jsonl"));
    heard(&mut state, "/t/a.jsonl", "before");
    state.follow(&session, PathBuf::from("/t/b.jsonl"));
    heard(&mut state, "/t/a.jsonl", "late");
    heard(&mut state, "/t/b.jsonl", "after");
    let chat = state.chat(&session).expect("a conversation");
    assert_eq!(chat.entries(), [Entry::You("after".into())]);
}

#[test]
fn an_ended_agent_forgets_its_conversation() {
    let (mut state, session) = (state(), SessionId::new("s"));
    state.follow(&session, PathBuf::from("/t/a.jsonl"));
    state.end(&session);
    assert!(state.chat(&session).is_none());
    assert!(
        state
            .follow(&session, PathBuf::from("/t/a.jsonl"))
            .is_none()
    );
}
