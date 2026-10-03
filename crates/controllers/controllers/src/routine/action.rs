//! The actions Groove does itself for an action routine, with no agent of the routine's own.

use groove_types::{Action, Timestamp};

use crate::session::Start;
use crate::{AppState, Services, Spawner};

pub(super) fn act(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    action: Action,
) {
    let opened = match action {
        Action::StartDue => start_due(state, services, spawner),
    };
    super::trail::acted(state, action, &opened);
}

/// Up Next's tasks that must start today, opened beside the selection with `groove:start-task`.
fn start_due(state: &mut AppState, services: &Services, spawner: &dyn Spawner) -> Vec<String> {
    let cap = state.config.preferences().routine_cap as usize;
    let worked = state.session.worked();
    let due = state.task.due_today(&worked, Timestamp::now().day(), cap);
    for short_id in &due {
        let start = Start {
            prompt: Some("/groove:start-task".to_string()),
            beside: true,
        };
        crate::task::open_with(state, services, spawner, short_id, start);
    }
    due
}
