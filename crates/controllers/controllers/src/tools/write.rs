//! A write the agent asks for: it waits for a human, then it runs.

use groove_agent_service::tools::Arguments;
use groove_agent_service::{Call, NewAsk, Reply};
use groove_types::{Approval, ApprovalId, Origin, SessionId, Timestamp};

use crate::workspace::git::Remote;
use crate::{AppState, Services, Spawner};
use groove_delivery_service::MrAct;

mod forge;
mod git;
mod notes;
mod repos;
mod skills;
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
        self.arguments.text(name)
    }

    pub(crate) fn number(&self, name: &str) -> Option<i64> {
        self.arguments.number(name)
    }

    pub(crate) fn flag(&self, name: &str) -> Option<bool> {
        self.arguments.flag(name)
    }
}

/// A write waits for a human, unless the session approves its own.
pub(super) fn asked(state: &mut AppState, services: &Services, spawner: &dyn Spawner, call: Call) {
    let session = super::session(&call);
    let auto = state
        .session
        .get(&session)
        .is_some_and(|open| open.state.auto_approve);
    if auto {
        return run(state, services, spawner, Write::of(call));
    }
    if call.tool == "git_push" {
        return git::push_asked(state, spawner, Write::of(call));
    }
    queued(state, Write::of(call));
}

fn queued(state: &mut AppState, write: Write) {
    let new = NewAsk {
        session: write.session,
        op: write.tool,
        payload: write.arguments,
        origin: Origin::Mcp,
        at: Timestamp::now(),
    };
    state.agent.asked(new, write.reply);
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

/// Every write of a session that ends, refused.
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
        "create_mr" => forge::mr(state, services, spawner, write, MrAct::Open),
        "update_mr" => forge::mr(state, services, spawner, write, MrAct::Edit),
        "close_mr" => forge::mr(state, services, spawner, write, MrAct::Close),
        "comment_mr" => forge::comment(state, services, spawner, write),
        "create_annotation" => notes::create(state, services, spawner, write),
        "update_annotation" => notes::update(state, services, spawner, write),
        "resolve_annotation" => notes::resolve(state, services, spawner, write),
        "post_annotation" => notes::post(state, services, spawner, write),
        "reply_thread" => notes::reply(state, services, spawner, write),
        "resolve_thread" => notes::resolve_thread(state, services, spawner, write),
        "add_task_repo" => repos::add_repo(state, services, spawner, write),
        "add_task_worktree" => repos::add_worktree(state, services, spawner, write),
        "log_task_hours" => task::log_hours(state, services, spawner, write),
        "finish_task" => task::finish(state, services, spawner, write),
        "adopt_task" => task::adopt(state, spawner, write),
        "save_user_skill" => skills::save(state, spawner, write),
        _ => write
            .reply
            .failed(format!("groove runs no {} yet", write.tool)),
    }
}
