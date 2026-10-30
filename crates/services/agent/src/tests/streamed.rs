//! A headless agent's asks and turns: what its stream says moves its status, and only its own launch's stream counts.

use groove_types::chat::Permission;
use groove_types::{AgentStatus, SessionId, Timestamp};

use crate::{Agent, Event, State, StreamHeard, apply};

fn state() -> State {
    let mut state = State::default();
    let agent = Agent {
        terminal: None,
        stream: None,
        permissions: Vec::new(),
        activity: crate::activity(AgentStatus::Working, Timestamp::new(100)),
        started_at: Timestamp::new(100),
        launch: 7,
    };
    state.agents.push((SessionId::new("s"), agent));
    state
}

fn hear(state: &mut State, launch: u64, heard: StreamHeard) {
    let event = Event::Streamed {
        session: SessionId::new("s"),
        launch,
        heard,
        at: Timestamp::new(200),
    };
    apply(state, event);
}

fn asks(id: &str) -> StreamHeard {
    StreamHeard::Asks(Permission {
        id: id.into(),
        tool: "Bash".into(),
        detail: Some("cargo test".into()),
        questions: Vec::new(),
        input: serde_json::json!({"command": "cargo test"}),
    })
}

fn status(state: &State) -> AgentStatus {
    state
        .activity(&SessionId::new("s"))
        .expect("an agent")
        .status
        .clone()
}

#[test]
fn an_ask_waits_on_the_user_and_its_answer_sets_the_agent_working_again() {
    let mut state = state();
    hear(&mut state, 7, asks("r1"));
    hear(&mut state, 7, asks("r2"));
    assert_eq!(status(&state), AgentStatus::Asking);
    let session = SessionId::new("s");
    let taken = state.answered(&session, "r1", Timestamp::new(300));
    assert_eq!(taken.map(|one| one.id), Some("r1".into()));
    assert_eq!(status(&state), AgentStatus::Asking, "r2 still waits");
    state.answered(&session, "r2", Timestamp::new(300));
    assert_eq!(status(&state), AgentStatus::Working);
    assert!(
        state
            .answered(&session, "r2", Timestamp::new(300))
            .is_none()
    );
}

#[test]
fn a_withdrawn_ask_goes_and_a_turn_s_end_clears_the_rest_and_settles_the_agent() {
    let mut state = state();
    hear(&mut state, 7, asks("r1"));
    hear(&mut state, 7, asks("r2"));
    hear(&mut state, 7, StreamHeard::Withdrawn("r1".into()));
    let session = SessionId::new("s");
    let left: Vec<_> = state
        .permissions(&session)
        .iter()
        .map(|one| one.id.clone())
        .collect();
    assert_eq!(left, ["r2"]);
    hear(&mut state, 7, StreamHeard::Settled);
    assert!(state.permissions(&session).is_empty());
    assert_eq!(status(&state), AgentStatus::Idle);
}

#[test]
fn a_finished_turn_stays_done_and_an_earlier_launch_s_stream_is_not_heard() {
    let mut state = state();
    hear(&mut state, 6, asks("r1"));
    assert!(state.permissions(&SessionId::new("s")).is_empty());
    state.agents[0].1.activity.status = AgentStatus::Done { seen: false };
    hear(&mut state, 7, StreamHeard::Settled);
    assert_eq!(status(&state), AgentStatus::Done { seen: false });
}
