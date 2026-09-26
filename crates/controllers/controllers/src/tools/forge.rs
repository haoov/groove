//! What the forge says of a worktree's MR, read live.

use groove_agent_service::Call;
use groove_workspace_service::Delivered;
use serde_json::{Value, json};

use crate::{AppState, Continuation, Services, Spawner};

/// Every thread of the MR, with who wrote each note and whether it is resolved.
pub(super) fn threads(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    call: Call,
) {
    live(
        state,
        services,
        spawner,
        call,
        |one| json!({ "mr": one.mr.url, "threads": one.read.threads }),
    );
}

/// The run on the MR's head commit, and where to read it.
pub(super) fn ci(state: &mut AppState, services: &Services, spawner: &dyn Spawner, call: Call) {
    live(
        state,
        services,
        spawner,
        call,
        |one| json!({ "mr": one.mr.url, "ci": one.read.ci }),
    );
}

/// The worktree's MR read from its forge, answered, and put on the surface too.
fn live(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    call: Call,
    said: fn(&Delivered) -> Value,
) {
    let Some(named) = super::worktree(state, &call) else {
        return call.reply.failed(super::NO_WORKTREE);
    };
    let Some((repo, worktree)) = crate::workspace::pair(state, &named.id) else {
        return call.reply.failed(super::NO_WORKTREE);
    };
    let remote = match groove_workspace_service::Service::remote(&repo) {
        Ok(remote) => remote,
        Err(e) => return call.reply.failed(e.message),
    };
    let (service, reply) = (services.workspace.clone(), call.reply);
    spawner.spawn(Box::pin(async move {
        let read = service.read(&remote, &repo, &worktree).await;
        Box::new(
            move |state: &mut AppState, services: &Services, spawner: &dyn Spawner| match read {
                Ok(Some(one)) => {
                    reply.json(&said(&one));
                    crate::workspace::mr::took(state, services, spawner, &worktree.id, one);
                }
                Ok(None) => reply.failed(format!("{} has no MR", worktree.branch)),
                Err(e) => reply.failed(e.message),
            },
        ) as Continuation
    }));
}
