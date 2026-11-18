//! The cluster write the selected session waits on: its change shown in place of the worktree's until it is answered.

use groove_types::{ApprovalId, Error, SessionId};
use serde_json::Value;

use crate::{AppState, Continuation, Services, Spawner};

/// The change of the cluster write the selected session waits on, shown; false when it waits on none.
pub(crate) fn show(state: &mut AppState, spawner: &dyn Spawner) -> bool {
    let Some(session) = state.session.selected.clone() else {
        return false;
    };
    let Some(approval) = state.agent.reviewed(&session) else {
        return false;
    };
    let id = approval.id.clone();
    if state.workspace.reviewed() == Some(id.as_str()) {
        return true;
    }
    let Some(pair) = pair_of(&approval.payload) else {
        return false;
    };
    spawner.spawn(Box::pin(async move {
        let prepared = groove_workspace_service::prepared(vec![pair]).await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            if !waits(state, &session, &id) || state.workspace.reviewed() == Some(id.as_str()) {
                return;
            }
            match prepared {
                Ok(prepared) => state.workspace.review(&id, prepared),
                Err(e) => state.failed(e),
            }
        }) as Continuation
    }));
    true
}

/// Refused while `session` waits on a cluster write: it is approved or refused first.
pub(crate) fn answer_first(state: &mut AppState, session: &SessionId) -> bool {
    let waiting = state.agent.reviewed(session).is_some();
    if waiting {
        let why = "approve or refuse the cluster write this session waits on first";
        state.failed(Error::invalid(why));
    }
    waiting
}

/// Whether the selected session is still `session`, and still waits on `id`.
fn waits(state: &AppState, session: &SessionId, id: &ApprovalId) -> bool {
    let selected = state.session.selected.as_ref() == Some(session);
    selected
        && state
            .agent
            .reviewed(session)
            .is_some_and(|one| &one.id == id)
}

/// The file the change stands as, its YAML before and after, from what the ask carries.
fn pair_of(payload: &Value) -> Option<(String, String, String)> {
    let change = &payload["change"];
    let text = |at: usize| change[at].as_str().map(str::to_string);
    Some((text(0)?, text(1)?, text(2)?))
}
