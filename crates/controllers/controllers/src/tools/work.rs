//! What the work has come to: its change, its commits, its merge request.

use std::path::Path;

use groove_agent_service::Call;
use groove_types::Worktree;
use groove_workspace_service::{COMMITS_MAX, commits, summary};
use serde_json::{Value, json};

use crate::{AppState, Continuation, Services, Spawner};

/// Every worktree of the task, and what changed in each against its base.
pub(super) fn diff(state: &AppState, spawner: &dyn Spawner, call: Call) {
    let Some(worktrees) = super::worktrees(state, &call) else {
        return call.reply.failed(super::NO_SESSION);
    };
    let reply = call.reply;
    spawner.spawn(Box::pin(async move {
        let mut out = Vec::new();
        for one in worktrees {
            let files = summary(Path::new(&one.path)).await.unwrap_or_default();
            out.push(json!({
                "worktree_id": one.id,
                "repo": one.repo,
                "branch": one.branch,
                "target_branch": one.base_ref,
                "files": files,
            }));
        }
        Box::new(move |_: &mut AppState, _: &Services, _: &dyn Spawner| {
            reply.json(&json!({ "worktrees": out }));
        }) as Continuation
    }));
}

/// The commits each worktree's branch holds over the branch it is based on.
pub(super) fn log(state: &AppState, spawner: &dyn Spawner, call: Call) {
    let Some(worktrees) = super::worktrees(state, &call) else {
        return call.reply.failed(super::NO_SESSION);
    };
    let limit = call
        .number("limit")
        .unwrap_or(20)
        .clamp(1, COMMITS_MAX as i64) as usize;
    let reply = call.reply;
    spawner.spawn(Box::pin(async move {
        let mut out = Vec::new();
        for one in worktrees {
            let dir = Path::new(&one.path);
            let log = commits(dir, one.base_ref.as_deref(), limit)
                .await
                .unwrap_or_default();
            out.push(json!({
                "worktree_id": one.id,
                "branch": one.branch,
                "commits": log,
            }));
        }
        Box::new(move |_: &mut AppState, _: &Services, _: &dyn Spawner| {
            reply.json(&json!({ "worktrees": out }));
        }) as Continuation
    }));
}

/// What one worktree has: its branch, what is changed, and how far it stands from origin.
pub(super) fn status(state: &AppState, services: &Services, spawner: &dyn Spawner, call: Call) {
    let Some(worktrees) = super::worktrees(state, &call) else {
        return call.reply.failed(super::NO_SESSION);
    };
    let (service, reply) = (services.session.clone(), call.reply);
    spawner.spawn(Box::pin(async move {
        let mut out = Vec::new();
        for one in worktrees {
            let told = service.status(&one).await.unwrap_or_default();
            out.push(told_status(&one, &told));
        }
        Box::new(move |_: &mut AppState, _: &Services, _: &dyn Spawner| {
            reply.json(&json!({ "worktrees": out }));
        }) as Continuation
    }));
}

fn told_status(one: &Worktree, told: &groove_types::WorktreeStatus) -> Value {
    json!({
        "worktree_id": one.id,
        "branch": one.branch,
        "target_branch": one.base_ref,
        "modified": told.modified,
        "staged": told.staged,
        "ahead": told.ahead,
        "behind": told.behind,
    })
}

/// The merge request one worktree has, as the app last stored it.
pub(super) fn mr(state: &AppState, services: &Services, spawner: &dyn Spawner, call: Call) {
    let Some(worktree) = super::worktree(state, &call) else {
        return call.reply.failed(super::NO_WORKTREE);
    };
    let (service, reply) = (services.workspace.clone(), call.reply);
    spawner.spawn(Box::pin(async move {
        let stored = service.stored(&worktree.id).await;
        Box::new(
            move |_: &mut AppState, _: &Services, _: &dyn Spawner| match stored {
                Ok(mr) => reply.json(&json!({ "mr": mr })),
                Err(e) => reply.failed(e.message),
            },
        ) as Continuation
    }));
}
