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
    if at.clone_url().is_none() {
        return state.failed(groove_session_service::unsourced(at));
    }
    let now = Timestamp::now();
    let session = review_session(at, now);
    let auto = session::begun(state, spawner, session.clone(), now);
    let spec = WorktreeSpec {
        branch: Some(at.source_branch.clone()),
        target: Some(at.target_branch.clone()),
        track_remote: Some(at.source_branch.clone()),
        mr_head: Some(at.head_ref()),
    };
    let (service, pending) = (
        services.session.clone(),
        state.begin(format!("checking out {}", at.project)),
    );
    let at = at.clone();
    session::added(spawner, (pending, None), Asker::Ui, async move {
        let added = service.open_review(&session, &at, &spec, now).await?;
        service.set_auto_approve(&session.id, auto).await?;
        Ok(added)
    });
}
