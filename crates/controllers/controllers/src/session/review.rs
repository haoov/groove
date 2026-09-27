//! Reviewing someone else's MR: the session, its repo, and its branch checked out.

use groove_session_service::review_session;
use groove_types::{ReviewMr, SessionId, Timestamp, WorktreeSpec};

use crate::asker::Asker;
use crate::{AppState, Services, Spawner, session};

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
    let Some(name) = repo_name(services, at) else {
        return state.failed(groove_types::Error::invalid(format!(
            "{} says nothing of where {} is cloned from",
            at.web_url, at.project
        )));
    };
    let now = Timestamp::now();
    let session = review_session(at, now);
    session::begun(state, spawner, session.clone(), now);
    let spec = WorktreeSpec {
        branch: Some(at.source_branch.clone()),
        target: Some(at.target_branch.clone()),
        track_remote: Some(at.source_branch.clone()),
    };
    let (service, pending) = (
        services.session.clone(),
        state.begin(format!("checking out {name}")),
    );
    session::added(spawner, pending, Asker::Ui, async move {
        service.open_review(&session, &name, &spec, now).await
    });
}

/// The pool's own name for the MR's repo, or the URL it must be cloned from.
fn repo_name(services: &Services, at: &ReviewMr) -> Option<String> {
    let pooled = services
        .session
        .list_pool()
        .iter()
        .find(|entry| entry.holds(&at.project))
        .map(|entry| entry.slug.clone());
    pooled.or_else(|| at.clone_url())
}
