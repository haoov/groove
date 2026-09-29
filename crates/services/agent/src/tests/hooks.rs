use groove_types::{AgentStatus, HookKind, SessionId, Timestamp, ToolCall};

use crate::{Agent, Event, State, apply};

fn state() -> State {
    let mut state = State::default();
    let agent = Agent {
        terminal: None,
        activity: crate::activity(AgentStatus::Idle, Timestamp::new(100)),
        started_at: Timestamp::new(100),
    };
    state.agents.push((SessionId::new("s"), agent));
    state
}

fn post(state: &mut State, kind: HookKind, tool: Option<ToolCall>, at: i64) {
    let event = Event::Hook {
        session: SessionId::new("s"),
        kind,
        tool,
        at: Timestamp::new(at),
    };
    apply(state, event);
}

fn tool() -> Option<ToolCall> {
    Some(ToolCall {
        name: "Bash".into(),
        detail: Some("cargo test --all".into()),
    })
}

#[test]
fn a_prompt_sets_the_agent_working_and_stop_sets_it_done() {
    let mut state = state();
    post(&mut state, HookKind::UserPromptSubmit, None, 200);
    let activity = state.activity(&SessionId::new("s")).expect("an agent");
    assert_eq!(activity.status, AgentStatus::Working);
    assert_eq!(activity.changed_at, Timestamp::new(200));

    post(&mut state, HookKind::Stop, None, 300);
    let activity = state.activity(&SessionId::new("s")).expect("an agent");
    assert_eq!(activity.status, AgentStatus::Done { seen: false });
    assert_eq!(activity.changed_at, Timestamp::new(300));
}

#[test]
fn a_tool_is_shown_while_it_runs_and_forgotten_after() {
    let mut state = state();
    post(&mut state, HookKind::PreToolUse, tool(), 200);
    let activity = state.activity(&SessionId::new("s")).expect("an agent");
    assert_eq!(activity.status, AgentStatus::Working);
    assert_eq!(activity.tool, tool());

    post(&mut state, HookKind::PostToolUse, tool(), 210);
    let activity = state.activity(&SessionId::new("s")).expect("an agent");
    assert_eq!(activity.status, AgentStatus::Working);
    assert!(activity.tool.is_none(), "the tool finished");
}

#[test]
fn a_run_of_tool_hooks_leaves_the_time_where_the_turn_started() {
    let mut state = state();
    post(&mut state, HookKind::UserPromptSubmit, None, 200);
    post(&mut state, HookKind::PreToolUse, tool(), 260);
    post(&mut state, HookKind::PostToolUse, tool(), 280);
    let activity = state.activity(&SessionId::new("s")).expect("an agent");
    assert_eq!(
        activity.changed_at,
        Timestamp::new(200),
        "the row says how long the turn has run, not how long the last tool took"
    );
}

#[test]
fn a_question_waits_on_the_user_until_it_is_answered() {
    let mut state = state();
    let question = Some(ToolCall {
        name: "AskUserQuestion".into(),
        detail: None,
    });
    post(&mut state, HookKind::UserPromptSubmit, None, 200);
    post(&mut state, HookKind::PreToolUse, question.clone(), 250);
    let status = |state: &State| {
        state
            .activity(&SessionId::new("s"))
            .expect("an agent")
            .status
            .clone()
    };
    assert_eq!(status(&state), AgentStatus::Asking);
    post(&mut state, HookKind::PostToolUse, question, 280);
    assert_eq!(status(&state), AgentStatus::Working, "answered");
}

#[test]
fn a_prompt_the_terminal_shows_waits_on_the_user_too() {
    let mut state = state();
    post(&mut state, HookKind::UserPromptSubmit, None, 200);
    post(&mut state, HookKind::Notification, None, 250);
    let activity = state.activity(&SessionId::new("s")).expect("an agent");
    assert_eq!(activity.status, AgentStatus::Asking);
    assert_eq!(activity.changed_at, Timestamp::new(250));
}

#[test]
fn a_hook_for_an_agent_that_is_gone_is_dropped() {
    let mut state = State::default();
    post(&mut state, HookKind::UserPromptSubmit, None, 200);
    assert!(state.activity(&SessionId::new("s")).is_none());
}

#[test]
fn a_session_posts_its_hooks_to_its_own_url() {
    let session = groove_types::Session {
        id: SessionId::new("gh-groove-50"),
        title: "Harden Groove".into(),
        kind: groove_types::SessionKind::Explorer,
        created_at: Timestamp::new(0),
    };
    let paths = crate::LaunchPaths {
        hooks: Some(crate::Receiver {
            port: 41234,
            token: "s3cr3t".into(),
        }),
        tools: Some(crate::Server {
            port: 41823,
            token: "t00l".into(),
        }),
        ..Default::default()
    };
    let loopback = crate::launch::loopback(&paths, &session).expect("a loopback");
    assert_eq!(
        loopback.hook_url,
        "http://127.0.0.1:41234/hook/gh-groove-50"
    );
    assert_eq!(loopback.token, "s3cr3t");
    let tools = loopback.tools.expect("the tool server");
    assert_eq!(
        tools.sse_url,
        "http://127.0.0.1:41823/sse?task=gh-groove-50"
    );
    assert_eq!(tools.token, "t00l");
    assert!(
        crate::launch::loopback(&crate::LaunchPaths::default(), &session).is_none(),
        "no receiver, no hooks"
    );
}
