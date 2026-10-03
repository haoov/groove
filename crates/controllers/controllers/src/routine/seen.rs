//! What the triggers read of the state, gathered from the slices that hold it.

use groove_agent_service::runs::{Place, Seen, Sessions};
use groove_types::AgentStatus;

use crate::AppState;

/// The state as the triggers read it; `before` carries what only a change can tell.
pub(super) fn look(state: &AppState, before: Option<&Seen>) -> Seen {
    let (owners, reading) = (state.session.owners(), state.task.reading);
    let worktrees: Vec<_> = owners.keys().cloned().collect();
    let (changes, commented) = state.delivery.reviewed(&worktrees);
    Seen {
        selected: state.session.selected.clone(),
        ci: state.delivery.ci_of(&worktrees),
        changes,
        commented,
        asked: state.delivery.asked(),
        working: state
            .agent
            .sessions_where(|one| *one == AgentStatus::Working),
        done: state
            .agent
            .sessions_where(|one| matches!(one, AgentStatus::Done { .. })),
        tasks: state.task.statuses(),
        tasks_read: before.is_some_and(|one| one.tasks_read || one.reading && !reading),
        reading,
        owners,
    }
}

/// The rail as the run rules weigh it.
pub(super) fn sessions(state: &AppState) -> Sessions {
    let place = |open: &groove_session_service::Open| Place {
        id: open.session.id.clone(),
        routine: open.session.kind.routine().map(str::to_string),
        auto_approve: open.state.auto_approve,
    };
    Sessions {
        open: state.session.open.iter().map(place).collect(),
        selected: state.session.selected.clone(),
    }
}
