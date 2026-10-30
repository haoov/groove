//! The agent's conversation: its hooks name the transcript, the transcript is followed, and the user speaks to it.

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

/// The turn stopped: a control message to a headless agent, Escape to one on a terminal.
pub(super) fn interrupt(state: &mut AppState, session: &SessionId) {
    match state.agent.stream(session) {
        Some(stream) => drop(stream.interrupt()),
        None => super::send(state, session, b"\x1b"),
    }
}

/// The ask `id` answered: the tool runs on the picked answers, or the agent hears it may not.
pub(super) fn answer(
    state: &mut AppState,
    session: &SessionId,
    id: &str,
    (allow, picked): (bool, &[String]),
) {
    let Some(asked) = state.agent.answered(session, id, Timestamp::now()) else {
        return;
    };
    let Some(stream) = state.agent.stream(session) else {
        return;
    };
    let _ = match allow {
        true => stream.allow(id, asked.answered(picked)),
        false => stream.deny(id, REFUSED),
    };
}

/// What a headless agent hears when the user refuses a tool.
const REFUSED: &str =
    "The user refused this tool call. Do not run it again; ask what to do instead.";
