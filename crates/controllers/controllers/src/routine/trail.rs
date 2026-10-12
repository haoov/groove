//! What a run leaves on the feed: when it started and why, how it ended and the commits it made.

use groove_agent_service::runs::{Ending, Run};
use groove_types::{Action, TimelineKind};

use crate::{AppState, Services, Spawner, Told};

/// `fix-mr · CI failed on main`, on the session it runs on.
pub(super) fn started(state: &AppState, services: &Services, spawner: &dyn Spawner, run: &Run) {
    let by = match (run.about.is_empty(), run.trigger) {
        (false, _) => run.about.clone(),
        (true, by) => groove_agent_service::routines::started_by(by).to_string(),
    };
    let subject = format!("{} · {by}", name(state, run));
    crate::timeline::said(
        services,
        spawner,
        &run.session,
        TimelineKind::RoutineRan,
        subject,
    );
}

/// `fix-mr · done · 2 commits`, on the session it ran on.
pub(super) fn ended(
    state: &AppState,
    services: &Services,
    spawner: &dyn Spawner,
    (run, ending): &(Run, Ending),
) {
    let commits = run
        .sent_at
        .map_or(0, |sent| state.session.commits_since(&run.session, sent));
    let subject = format!("{} · {}", name(state, run), ending.said(commits));
    crate::timeline::said(
        services,
        spawner,
        &run.session,
        TimelineKind::RoutineEnded,
        subject,
    );
}

/// `start-due opened T-1, T-2` as a note; nothing when it opened nothing.
pub(super) fn acted(state: &mut AppState, action: Action, opened: &[String]) {
    if opened.is_empty() {
        return;
    }
    let said = format!("{} opened {}", action.name(), opened.join(", "));
    state.notes.push(Told::now(said));
}

fn name(state: &AppState, run: &Run) -> String {
    let routine = state.agent.routine(&run.routine);
    routine.map_or_else(|| run.routine.clone(), |one| one.name.clone())
}
