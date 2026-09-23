//! What the agent asks Groove for, answered where the state is.

mod about;
mod files;
mod work;

use groove_agent_service::Call;
use groove_session_service::Open;
use groove_types::{ExternalId, SessionId, Worktree, WorktreeId};

use crate::{AppState, Services, Spawner};

pub(crate) const NO_SESSION: &str = "this call belongs to no open session";
pub(crate) const NO_WORKTREE: &str = "no worktree of an open session has that id";

/// The call answered from the state, or handed to a job that answers later.
pub fn answer(state: &mut AppState, services: &Services, spawner: &dyn Spawner, call: Call) {
    match call.tool.as_str() {
        "get_active_task" => about::active(state, call),
        "list_tasks" => about::tasks(state, call),
        "list_repos" => about::repos(state, services, call),
        "get_task_diff" => work::diff(state, spawner, call),
        "get_commit_log" => work::log(state, spawner, call),
        "get_status" => work::status(state, services, spawner, call),
        "get_mr_state" => work::mr(state, services, spawner, call),
        "get_open_file" => files::open_file(state, call),
        "read_file" => files::read(state, spawner, call),
        tool => call.reply.failed(format!("groove answers no {tool} yet")),
    }
}

/// The session the call was made from.
pub(crate) fn session(call: &Call) -> SessionId {
    SessionId::new(&call.session)
}

/// The session a read is about: the one its `task_id` names, or the call's own.
pub(crate) fn about<'a>(state: &'a AppState, call: &Call) -> Option<&'a Open> {
    let Some(task) = call.text("task_id") else {
        return state.session.get(&session(call));
    };
    let external = state
        .task
        .get(task)
        .map(|one| one.external_id.clone())
        .unwrap_or_else(|| ExternalId::new(task));
    state
        .session
        .open
        .iter()
        .find(|open| open.session.kind.works(&external))
}

/// Every worktree of that session.
pub(crate) fn worktrees(state: &AppState, call: &Call) -> Option<Vec<Worktree>> {
    about(state, call).map(|open| open.worktrees.clone())
}

/// The worktree a call names, in whichever open session holds it.
pub(crate) fn worktree(state: &AppState, call: &Call) -> Option<Worktree> {
    let id = WorktreeId::new(call.text("worktree_id")?);
    state
        .session
        .open
        .iter()
        .flat_map(|open| open.worktrees.iter())
        .find(|one| one.id == id)
        .cloned()
}
