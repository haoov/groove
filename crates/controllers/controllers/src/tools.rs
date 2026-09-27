//! What the agent asks Groove for, answered where the state is.

mod about;
mod files;
mod filing;
mod forge;
mod work;
mod write;

use groove_agent_service::Call;
use groove_session_service::Open;
use groove_types::{ExternalId, SessionId, Worktree, WorktreeId};

use crate::{AppState, Services, Spawner};

pub(crate) const NO_SESSION: &str = "this call belongs to no open session";
pub(crate) const NO_WORKTREE: &str = "no worktree of an open session has that id";
pub(crate) const NO_TASK: &str = "this session works no task, and none was named";

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
        "get_mr_threads" => forge::threads(state, services, spawner, call),
        "get_mr_ci" => forge::ci(state, services, spawner, call),
        "get_annotations" => files::notes(state, services, spawner, call),
        "list_skills" => about::skills(state, call),
        "read_user_skill" => about::skill(state, call),
        "get_task_body" => about::body(state, spawner, call),
        "get_task_template" => about::template(state, spawner, call),
        "get_open_file" => files::open_file(state, call),
        "read_file" => files::read(state, spawner, call),
        tool => match groove_agent_service::tools::named(tool) {
            Some(one) if one.writes => write::asked(state, services, spawner, call),
            _ => call.reply.failed(format!("groove answers no {tool} yet")),
        },
    }
}

pub use write::{allow, drop_asks, refuse};

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
    state.session.working(&external)
}

/// The task a read is about: the one its `task_id` names, or the session's own.
pub(crate) fn task_of(state: &AppState, call: &Call) -> Option<ExternalId> {
    let Some(named) = call.text("task_id") else {
        return about(state, call).and_then(|open| open.session.kind.task().cloned());
    };
    let known = state.task.get(named).map(|one| one.external_id.clone());
    Some(known.unwrap_or_else(|| ExternalId::new(named)))
}

/// Every worktree of that session.
pub(crate) fn worktrees(state: &AppState, call: &Call) -> Option<Vec<Worktree>> {
    about(state, call).map(|open| open.worktrees.clone())
}

/// The worktree a call names, in whichever open session holds it.
pub(crate) fn worktree(state: &AppState, call: &Call) -> Option<Worktree> {
    worktree_named(state, call.text("worktree_id"))
}

/// The same, by the id itself.
pub(crate) fn worktree_named(state: &AppState, named: Option<&str>) -> Option<Worktree> {
    let (_, _, worktree) = state.session.find(&WorktreeId::new(named?))?;
    Some(worktree.clone())
}
