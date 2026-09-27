//! What the tasks mean once they are read: the sessions' start days and MRs, folded.

use groove_types::{Day, ExternalId, MrFacts, Timestamp};

use crate::AppState;

/// The tasks read again against the sessions on disk and the MRs of the ones on the rail.
pub(crate) fn reread(state: &mut AppState, now: Timestamp) {
    let started = started(state);
    let mrs = mrs(state);
    let thresholds = state.config.thresholds();
    state.task.reread(started, mrs, now, &thresholds);
}

/// The day each task's sessions opened.
fn started(state: &AppState) -> Vec<(ExternalId, Day)> {
    let living = state.session.living.iter();
    living
        .filter_map(|one| {
            Some((
                one.session.kind.task()?.clone(),
                one.session.created_at.day(),
            ))
        })
        .collect()
}

/// What each MR of a task's session stands at.
fn mrs(state: &AppState) -> Vec<(ExternalId, MrFacts)> {
    let mut out = Vec::new();
    for open in &state.session.open {
        let Some(task) = open.session.kind.task() else {
            continue;
        };
        for worktree in &open.worktrees {
            if let Some(facts) = state
                .delivery
                .held(&worktree.id)
                .and_then(|one| one.facts())
            {
                out.push((task.clone(), facts));
            }
        }
    }
    out
}
