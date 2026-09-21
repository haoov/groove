//! Reviewing someone else's MR: the session, its repo, and its branch checked out.

use groove_session_service::review_session;
use groove_types::{ReviewMr, SessionId, Timestamp, WorktreeSpec};

use crate::{AppState, Services, Spawner, agent, session};

/// The session that reviews this MR: the one it has, or a new one on the MR's branch.
pub fn open_review(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    at: &ReviewMr,
) {
    let id = SessionId::new(at.session_id());
    if state.session.get(&id).is_some() {
        return session::select(state, services, spawner, &id);
    }
    let Some(name) = repo_name(state, at) else {
        return state.errors.push(groove_types::Error::invalid(format!(
            "{} says nothing of where {} is cloned from",
            at.web_url, at.project
        )));
    };
    let now = Timestamp::now();
    let session = review_session(at, now);
    state.session.open(session.clone(), now);
    crate::workspace::follow(state, spawner);
    agent::start(state, spawner, id.clone(), session::FIRST_SIZE);
    let spec = WorktreeSpec {
        branch: Some(at.source_branch.clone()),
        target: Some(at.target_branch.clone()),
        track_remote: Some(at.source_branch.clone()),
    };
    let service = services.session.clone();
    session::listed(spawner, session::NO_PENDING, async move {
        service.create_review(&session, now).await
    });
    session::add_repo(state, services, spawner, &id, &name, spec);
}

/// The pool's own name for the MR's repo, or the URL it must be cloned from.
fn repo_name(state: &AppState, at: &ReviewMr) -> Option<String> {
    let pooled = state
        .session
        .pool
        .iter()
        .find(|entry| entry.slug.ends_with(&at.project))
        .map(|entry| entry.slug.clone());
    pooled.or_else(|| at.clone_url())
}
