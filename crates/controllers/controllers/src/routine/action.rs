//! The actions Groove does itself for an action routine, with no agent of the routine's own.

use groove_types::{Action, Day, Task, Timestamp};

use crate::{AppState, Services, Spawner};

/// The working hours a day of estimate stands for.
const HOURS_A_DAY: f32 = 8.0;

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

/// Up Next's tasks that must start today, opened with `groove:start-task`, the cap's worth at most.
fn start_due(state: &mut AppState, services: &Services, spawner: &dyn Spawner) -> Vec<String> {
    let today = Timestamp::now().day();
    let cap = state.config.preferences().routine_cap as usize;
    let worked = state.session.worked();
    let waiting = state.task.waiting(&worked);
    let due: Vec<String> = state
        .task
        .planned(&waiting)
        .into_iter()
        .filter(|one| !one.later && starts(one.task, today))
        .map(|one| one.task.short_id.clone())
        .take(cap)
        .collect();
    let selected = state.session.selected.clone();
    for short_id in &due {
        let prompt = Some("/groove:start-task".to_string());
        crate::task::open_asking(state, services, spawner, short_id, prompt);
    }
    if let Some(id) = selected {
        state.session.selected = Some(id);
        crate::task::follow(state, spawner);
        crate::workspace::follow(state, spawner);
    }
    due
}

/// Its Start date has come, or today plus its estimate reaches its Due date.
pub(super) fn starts(task: &Task, today: Day) -> bool {
    let started = task
        .dates
        .start
        .is_some_and(|start| start.days() <= today.days());
    let hours = task.estimate.unwrap_or_default().max(0.0);
    let needed = (hours / HOURS_A_DAY).ceil() as i64;
    let pressed = task
        .dates
        .due
        .is_some_and(|due| today.days() + needed >= due.days());
    started || pressed
}
