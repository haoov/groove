//! What the session itself is: its own task, the tasks it knows, the repos it may take.

use groove_agent_service::Call;
use groove_types::{ProviderId, Repo, Task, TaskKey, Worktree};
use serde_json::{Value, json};

use crate::{AppState, Continuation, Services, Spawner};

/// The session the call comes from, with its repos and its worktrees.
pub(super) fn active(state: &AppState, call: Call) {
    let Some(open) = state.session.get(&super::session(&call)) else {
        return call.reply.failed(super::NO_SESSION);
    };
    let said = json!({
        "session": open.session,
        "auto_approve": open.state.auto_approve,
        "repos": open.repos.iter().map(repo).collect::<Vec<Value>>(),
        "worktrees": open.worktrees.iter().map(worktree).collect::<Vec<Value>>(),
    });
    call.reply.json(&said);
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
    let sources = groove_task_service::sources(state.config.config.as_ref());
    let reply = call.reply;
    spawner.spawn(Box::pin(async move {
        let read = groove_task_service::fetch(&sources, &key).await;
        Box::new(
            move |state: &mut AppState, _: &Services, _: &dyn Spawner| match read {
                Ok(read) => {
                    reply.json(&json!({
                        "task_id": read.task.short_id,
                        "title": read.task.title,
                        "url": read.task.url,
                        "body_markdown": read.body,
                    }));
                    state.task.synced(read);
                }
                Err(e) => reply.failed(e.to_string()),
            },
        ) as Continuation
    }));
}

/// The headings a new task starts from, from the source that holds them.
pub(super) fn template(state: &AppState, spawner: &dyn Spawner, call: Call) {
    let which = match call.text("provider").map(ProviderId::parse).transpose() {
        Ok(which) => which,
        Err(e) => return call.reply.failed(e.to_string()),
    };
    let sources = groove_task_service::sources(state.config.config.as_ref());
    let file_at = super::filing::file_at(state.config.config.as_ref(), which);
    let reply = call.reply;
    spawner.spawn(Box::pin(async move {
        let read = groove_task_service::template(&sources, which).await;
        Box::new(
            move |_: &mut AppState, _: &Services, _: &dyn Spawner| match read {
                Ok(held) => reply.json(&json!({
                    "template_markdown": held.unwrap_or_default(),
                    "file_at": file_at,
                })),
                Err(e) => reply.failed(e.to_string()),
            },
        ) as Continuation
    }));
}

/// Every task the app has read, explorers and reviews aside.
pub(super) fn tasks(state: &AppState, call: Call) {
    let tasks: Vec<Value> = state.task.tasks.iter().map(task).collect();
    call.reply
        .json(&json!({ "count": tasks.len(), "tasks": tasks }));
}

/// Every clone in the pool, and whether this session already has it.
pub(super) fn repos(state: &AppState, services: &Services, call: Call) {
    let held: Vec<String> = match state.session.get(&super::session(&call)) {
        Some(open) => open.repos.iter().map(Repo::slug).collect(),
        None => Vec::new(),
    };
    let pool: Vec<Value> = services
        .session
        .list_pool()
        .iter()
        .map(|entry| {
            json!({
                "repo": entry.slug,
                "local_path": entry.path,
                "attached": held.contains(&entry.slug),
            })
        })
        .collect();
    call.reply
        .json(&json!({ "count": pool.len(), "repos": pool }));
}

/// Every skill this session can be sent, with what each one is for.
pub(super) fn skills(state: &AppState, call: Call) {
    let Some(open) = state.session.get(&super::session(&call)) else {
        return call.reply.failed(super::NO_SESSION);
    };
    let offered: Vec<Value> = state
        .agent
        .skills_for(&open.session.kind)
        .iter()
        .map(|one| {
            json!({
                "id": one.id,
                "name": one.name,
                "description": one.description,
                "yours": one.editable,
            })
        })
        .collect();
    call.reply
        .json(&json!({ "count": offered.len(), "skills": offered }));
}

/// One skill of the user's own, as its file stands.
pub(super) fn skill(state: &AppState, call: Call) {
    let Some(name) = call.text("name") else {
        return call.reply.failed("read_user_skill needs a name");
    };
    let dirs = crate::agent::skills::dirs(state);
    match groove_agent_service::skills::read(&dirs, &format!("user:{name}")) {
        Ok(body) => call.reply.said(body),
        Err(e) => call.reply.failed(e.message),
    }
}

fn repo(one: &Repo) -> Value {
    json!({
        "id": one.id,
        "repo": one.slug(),
        "project": one.project,
        "local_path": one.local_path,
    })
}

fn worktree(one: &Worktree) -> Value {
    json!({
        "worktree_id": one.id,
        "repo": one.repo,
        "branch": one.branch,
        "target_branch": one.base_ref,
        "path": one.path,
    })
}

fn task(one: &Task) -> Value {
    json!({
        "task_id": one.short_id,
        "external_id": one.external_id,
        "title": one.title,
        "status": one.status,
        "priority": one.priority,
        "url": one.url,
    })
}
