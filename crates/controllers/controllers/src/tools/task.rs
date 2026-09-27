//! What the agent reads of tasks: the ones known, one task's page, the template to file by.

use groove_agent_service::Call;
use groove_agent_service::tools::answers;
use groove_types::{ProviderId, TaskKey};

use crate::{AppState, Continuation, Services, Spawner};

/// Every task the app has read, explorers and reviews aside.
pub(super) fn tasks(state: &AppState, call: Call) {
    let each = state.task.tasks.iter().map(answers::task).collect();
    call.reply.json(&answers::counted("tasks", each));
}

/// The task's page, as its source holds it now.
pub(super) fn body(state: &AppState, spawner: &dyn Spawner, call: Call) {
    let Some(id) = super::task_of(state, &call) else {
        return call.reply.failed(super::NO_TASK);
    };
    let key = match TaskKey::parse(&id) {
        Ok(key) => key,
        Err(e) => return call.reply.failed(e.to_string()),
    };
    let sources = state.task.sources(state.config.config.as_ref());
    let reply = call.reply;
    spawner.spawn(Box::pin(async move {
        let read = groove_task_service::fetch(&sources, &key).await;
        Box::new(
            move |state: &mut AppState, _: &Services, _: &dyn Spawner| match read {
                Ok(read) => {
                    reply.json(&answers::body(&read.task, &read.body));
                    state.task.synced(read);
                }
                Err(e) => reply.failed(e.to_string()),
            },
        ) as Continuation
    }));
}

/// The headings a new task starts from, and where it is filed.
pub(super) fn template(state: &AppState, spawner: &dyn Spawner, call: Call) {
    let which = match call.text("provider").map(ProviderId::parse).transpose() {
        Ok(which) => which,
        Err(e) => return call.reply.failed(e.to_string()),
    };
    let sources = state.task.sources(state.config.config.as_ref());
    let file_at = groove_task_service::filing(state.config.config.as_ref(), which);
    let reply = call.reply;
    spawner.spawn(Box::pin(async move {
        let read = groove_task_service::template(&sources, which).await;
        Box::new(
            move |_: &mut AppState, _: &Services, _: &dyn Spawner| match read {
                Ok(held) => reply.json(&answers::template(&held.unwrap_or_default(), file_at)),
                Err(e) => reply.failed(e.to_string()),
            },
        ) as Continuation
    }));
}
