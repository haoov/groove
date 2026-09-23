//! A write the agent asks for: it waits for a human, then it runs.

use groove_agent_service::{Call, NewAsk, Reply};
use groove_types::{Approval, ApprovalId, Origin, SessionId, Timestamp};

use crate::workspace::git::Remote;
use crate::{AppState, Services, Spawner};

mod forge;
mod git;
mod notes;
mod repos;
mod task;

/// One write, from the call that asked for it or the approval that let it through.
pub(crate) struct Write {
    pub session: SessionId,
    pub tool: String,
    pub arguments: serde_json::Value,
    pub reply: Reply,
}

impl Write {
    fn of(call: Call) -> Self {
        Self {
            session: SessionId::new(&call.session),
            tool: call.tool,
            arguments: call.arguments,
            reply: call.reply,
        }
    }

    fn approved(approval: Approval, reply: Reply) -> Option<Self> {
        Some(Self {
            session: approval.session?,
            tool: approval.op,
            arguments: approval.payload,
            reply,
        })
    }

    pub(crate) fn text(&self, name: &str) -> Option<&str> {
        self.arguments[name].as_str().filter(|one| !one.is_empty())
    }

    pub(crate) fn number(&self, name: &str) -> Option<i64> {
        self.arguments[name].as_i64()
    }

    pub(crate) fn flag(&self, name: &str) -> Option<bool> {
        self.arguments[name].as_bool()
    }
}

/// A write waits for a human, unless the session approves its own.
pub(super) fn asked(state: &mut AppState, services: &Services, spawner: &dyn Spawner, call: Call) {
    let session = super::session(&call);
    let auto = state
        .agent
        .activity(&session)
        .is_some_and(|activity| activity.auto_approve);
    if auto {
        return run(state, services, spawner, Write::of(call));
    }
    let new = NewAsk {
        session,
        op: call.tool.clone(),
        payload: call.arguments.clone(),
        origin: Origin::Mcp,
        at: Timestamp::now(),
    };
    state.agent.asked(new, call.reply);
}

/// The write the user allowed, run at last.
pub fn allow(state: &mut AppState, services: &Services, spawner: &dyn Spawner, id: &ApprovalId) {
    let Some((approval, reply)) = state.agent.resolved(id) else {
        return;
    };
    let Some(write) = Write::approved(approval, reply) else {
        return;
    };
    run(state, services, spawner, write);
}

/// The write the user refused, which its agent hears about.
pub fn refuse(state: &mut AppState, id: &ApprovalId) {
    let Some((approval, reply)) = state.agent.resolved(id) else {
        return;
    };
    reply.failed(format!("the user refused {}", approval.op));
}

/// Every write of a session that ends, refused so no agent waits on it.
pub fn drop_asks(state: &mut AppState, session: &SessionId) {
    for (approval, reply) in state.agent.forget_asks(session) {
        reply.failed(format!("the session closed before {} ran", approval.op));
    }
}

/// The worktree the write names, or the one its own session has selected.
pub(crate) fn worktree_of(state: &AppState, write: &Write) -> Option<groove_types::Worktree> {
    let named = crate::tools::worktree_named(state, write.text("worktree_id"));
    named.or_else(|| {
        state
            .session
            .get(&write.session)?
            .selected_worktree()
            .cloned()
    })
}

fn run(state: &mut AppState, services: &Services, spawner: &dyn Spawner, write: Write) {
    match write.tool.as_str() {
        "git_commit" => git::commit(state, spawner, write),
        "git_push" => git::remote(state, services, spawner, write, Remote::Push),
        "git_pull" => git::remote(state, services, spawner, write, Remote::Pull),
        "create_mr" => forge::mr(state, services, spawner, write, forge::Act::Open),
        "update_mr" => forge::mr(state, services, spawner, write, forge::Act::Edit),
        "close_mr" => forge::mr(state, services, spawner, write, forge::Act::Close),
        "comment_mr" => forge::comment(state, services, spawner, write),
        "create_annotation" => notes::create(state, services, spawner, write),
        "update_annotation" => notes::on_note(state, services, spawner, write, false),
        "resolve_annotation" => notes::on_note(state, services, spawner, write, true),
        "post_annotation" => notes::post(state, services, spawner, write),
        "reply_thread" => notes::thread(state, services, spawner, write, true),
        "resolve_thread" => notes::thread(state, services, spawner, write, false),
        "add_task_repo" => repos::add_repo(state, services, spawner, write),
        "add_task_worktree" => repos::add_worktree(state, services, spawner, write),
        "log_task_hours" => task::log_hours(state, services, spawner, write),
        "finish_task" => task::finish(state, services, spawner, write),
        _ => write
            .reply
            .failed(format!("groove runs no {} yet", write.tool)),
    }
}
