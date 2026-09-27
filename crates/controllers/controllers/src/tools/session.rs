//! What the session itself is: its repos, its worktrees, the pool, its skills.

use groove_agent_service::Call;
use groove_agent_service::tools::answers;
use groove_types::Repo;

use crate::{AppState, Services};

/// The session the call comes from, with its repos and its worktrees.
pub(super) fn active(state: &AppState, call: Call) {
    let Some(open) = state.session.get(&super::session(&call)) else {
        return call.reply.failed(super::NO_SESSION);
    };
    let (auto, repos) = (open.state.auto_approve, &open.repos);
    call.reply.json(&answers::session(
        &open.session,
        auto,
        repos,
        &open.worktrees,
    ));
}

/// Every clone in the pool, and whether this session already has it.
pub(super) fn repos(state: &AppState, services: &Services, call: Call) {
    let held: Vec<String> = match state.session.get(&super::session(&call)) {
        Some(open) => open.repos.iter().map(Repo::slug).collect(),
        None => Vec::new(),
    };
    let pool = services.session.list_pool();
    let each = pool
        .iter()
        .map(|entry| answers::pooled(entry, held.contains(&entry.slug)))
        .collect();
    call.reply.json(&answers::counted("repos", each));
}

/// Every skill this session can be sent, with what each one is for.
pub(super) fn skills(state: &AppState, call: Call) {
    let Some(open) = state.session.get(&super::session(&call)) else {
        return call.reply.failed(super::NO_SESSION);
    };
    let offered = state.agent.skills_for(&open.session.kind);
    let each = offered.iter().map(|one| answers::skill(one)).collect();
    call.reply.json(&answers::counted("skills", each));
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
