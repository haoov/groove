//! What the tasks mean once they are read: their own start dates, and what asks.

use std::collections::BTreeMap;

use groove_types::{Day, ExternalId, MrFacts, Timestamp};

use crate::AppState;

/// The tasks as they now stand: their starts filled in, then what needs the user.
pub(crate) fn reread(state: &mut AppState, now: Timestamp) {
    start_dates(state);
    folded(state, now);
}

/// A task the source gives no start date starts the day its first session did.
fn start_dates(state: &mut AppState) {
    let mut first: std::collections::BTreeMap<ExternalId, Day> = std::collections::BTreeMap::new();
    for living in &state.session.living {
        let Some(external_id) = living.session.kind.task() else {
            continue;
        };
        let day = living.session.created_at.day();
        let held = first.entry(external_id.clone()).or_insert(day);
        *held = (*held).min(day);
    }
    for task in &mut state.task.tasks {
        if task.dates.start.is_none() {
            task.dates.start = first.get(&task.external_id).copied();
        }
    }
}

/// What needs the user, read again from the tasks as they now stand.
fn folded(state: &mut AppState, now: Timestamp) {
    let thresholds = state.config.thresholds();
    let facts = by_task(state);
    state.task.attention = groove_task_service::folded(&state.task.tasks, &facts, now, &thresholds);
}

/// Each worktree's MR against the task its session works, several as one.
fn by_task(state: &AppState) -> BTreeMap<ExternalId, MrFacts> {
    let mut out: BTreeMap<ExternalId, MrFacts> = BTreeMap::new();
    for open in &state.session.open {
        let Some(external_id) = open.session.kind.task() else {
            continue;
        };
        for worktree in &open.worktrees {
            let held = state.delivery.held(&worktree.id);
            let Some(facts) = held.and_then(groove_delivery_service::Held::facts) else {
                continue;
            };
            out.entry(external_id.clone())
                .and_modify(|held| *held = held.and(facts))
                .or_insert(facts);
        }
    }
    out
}
