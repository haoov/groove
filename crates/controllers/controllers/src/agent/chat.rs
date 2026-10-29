//! The agent's conversation: its hooks name the transcript, and the transcript is followed.

use std::path::PathBuf;

use groove_agent_service::{Event as AgentEvent, Post};
use groove_types::{SessionId, Timestamp};

use crate::{AppState, Event, Services, Spawner, apply};

/// One hook applied; the transcript it names is followed once it is new.
pub fn hooked(state: &mut AppState, spawner: &dyn Spawner, post: Post) {
    let (session, transcript) = (post.session.clone(), post.transcript);
    let event = AgentEvent::Hook {
        session: post.session,
        kind: post.kind,
        tool: post.tool,
        at: Timestamp::now(),
    };
    apply(Event::Agent(event), state);
    if let Some(path) = transcript {
        follow(state, spawner, session, path);
    }
}

fn follow(state: &mut AppState, spawner: &dyn Spawner, session: SessionId, path: PathBuf) {
    if state.agent.transcript(&session) == Some(path.as_path()) {
        return;
    }
    let Some(heeded) = state.agent.follow(&session, path.clone()) else {
        return;
    };
    let (sink, read) = (spawner.sink(), path.clone());
    let send = move |heard| {
        let (session, transcript) = (session.clone(), read.clone());
        sink.deliver(Box::new(
            move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
                let event = AgentEvent::Heard {
                    session,
                    transcript,
                    heard,
                };
                apply(Event::Agent(event), state)
            },
        ));
    };
    spawner.detach(Box::pin(groove_agent_service::follower(path, heeded, send)));
}

/// Words typed in Groove's own prompt, sent to the agent as if typed in its terminal, and shown waiting.
pub(super) fn say(state: &mut AppState, spawner: &dyn Spawner, session: &SessionId, text: &str) {
    let text = text.trim();
    if text.is_empty() {
        return;
    }
    super::skills::typed(state, spawner, session, text);
    let said = AgentEvent::Said {
        session: session.clone(),
        text: text.to_string(),
    };
    apply(Event::Agent(said), state);
}
