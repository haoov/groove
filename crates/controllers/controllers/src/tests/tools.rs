//! What the agent's tools answer, from a session the fixture opened.

mod adopt;
mod forge;
mod reads;
mod writes;

use groove_agent_service::{Answer, Call, Reply};
use serde_json::Value;

use crate::tests::fixture::{sh, worktree};
use crate::{AppState, Services, SyncSpawner};

/// One tool called as the agent of `session` would call it, and its answer.
pub(super) fn asked(
    state: &mut AppState,
    services: &Services,
    spawner: &SyncSpawner,
    session: &str,
    tool: &str,
    arguments: Value,
) -> Answer {
    waited(state, services, spawner, session, tool, arguments).expect("an answer")
}

/// The same, for a call that may still be waiting on the user.
pub(super) fn waited(
    state: &mut AppState,
    services: &Services,
    spawner: &SyncSpawner,
    session: &str,
    tool: &str,
    arguments: Value,
) -> Option<Answer> {
    let (reply, mut answered) = Reply::new();
    crate::tools::answer(
        state,
        services,
        spawner,
        Call {
            session: session.to_string(),
            tool: tool.to_string(),
            arguments,
            reply,
        },
    );
    for _ in 0..DRAINS {
        spawner.drain(state, services);
        if let Ok(answer) = answered.try_recv() {
            return Some(answer);
        }
    }
    None
}

/// How many rounds of jobs a tool may take to answer: a job's continuation can start another.
const DRAINS: usize = 20;

/// The answer, once the jobs behind it have landed.
pub(super) fn settled(
    spawner: &SyncSpawner,
    services: &Services,
    state: &mut AppState,
    answered: &mut tokio::sync::oneshot::Receiver<Answer>,
) -> Answer {
    for _ in 0..500 {
        spawner.drain(state, services);
        if let Ok(answer) = answered.try_recv() {
            return answer;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    panic!("no answer in five seconds");
}

/// Everything in the worktree staged, as `git add -A` would.
pub(super) fn stage(dir: &str) {
    sh(std::path::Path::new(dir), &["add", "-A"]);
}

/// What a tool that answers in data said.
pub(super) fn said(answer: &Answer) -> Value {
    assert!(!answer.failed, "{}", answer.text);
    serde_json::from_str(&answer.text).expect("json")
}

/// The approval that session waits on.
pub(super) fn waiting(state: &AppState, session: &str) -> Vec<groove_types::Ask> {
    state
        .agent
        .activity(&groove_types::SessionId::new(session))
        .map(|activity| activity.asks.clone())
        .unwrap_or_default()
}

/// A session with one worktree, and a file changed in it.
pub(super) fn changed(
    state: &mut AppState,
    services: &Services,
    spawner: &SyncSpawner,
) -> (String, String, String) {
    let dir = worktree(state, services, spawner);
    std::fs::write(std::path::Path::new(&dir).join("a.txt"), "one\ntwo\n").unwrap();
    let id = state.session.selected.clone().unwrap();
    let worktree = state
        .session
        .get(&id)
        .and_then(|open| open.selected_worktree())
        .unwrap();
    (
        id.as_str().to_string(),
        worktree.id.as_str().to_string(),
        dir,
    )
}
