//! What the work has come to: its change, its commits, its merge request.

use std::path::Path;

use groove_agent_service::Call;
use groove_agent_service::tools::answers;
use groove_workspace_service::{COMMITS_MAX, commits};

use crate::workspace::diff::{against, files_in};

use crate::{AppState, Continuation, Services, Spawner};

/// Every worktree of the task, and what its branch changed since its base.
pub(super) fn diff(state: &AppState, spawner: &dyn Spawner, call: Call) {
    let Some(worktrees) = super::worktrees(state, &call) else {
        return call.reply.failed(super::NO_SESSION);
    };
    let (mode, reply) = (groove_types::DiffMode::Base, call.reply);
    spawner.spawn(Box::pin(async move {
        let mut out = Vec::new();
        for one in worktrees {
            let dir = Path::new(&one.path);
            let rev = against(dir, mode, one.base_ref.as_deref()).await;
            let files = files_in(dir, mode, &rev).await.unwrap_or_default();
            out.push(answers::changed(&one, mode.label(), &files));
        }
        Box::new(move |_: &mut AppState, _: &Services, _: &dyn Spawner| {
            reply.json(&answers::worktrees(out));
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
            out.push(answers::commits(&one, &log));
        }
        Box::new(move |_: &mut AppState, _: &Services, _: &dyn Spawner| {
            reply.json(&answers::worktrees(out));
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
            out.push(answers::status(&one, &told));
        }
        Box::new(move |_: &mut AppState, _: &Services, _: &dyn Spawner| {
            reply.json(&answers::worktrees(out));
        }) as Continuation
    }));
}

/// The merge request one worktree has, as the app last stored it.
pub(super) fn mr(state: &AppState, services: &Services, spawner: &dyn Spawner, call: Call) {
    let Some(worktree) = super::worktree(state, &call) else {
        return call.reply.failed(super::NO_WORKTREE);
    };
    let (service, reply) = (services.delivery.clone(), call.reply);
    spawner.spawn(Box::pin(async move {
        let stored = service.stored(&worktree.id).await;
        Box::new(
            move |_: &mut AppState, _: &Services, _: &dyn Spawner| match stored {
                Ok(mr) => reply.json(&answers::mr(mr.as_ref())),
                Err(e) => reply.failed(e.message),
            },
        ) as Continuation
    }));
}
