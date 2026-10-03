//! What a run leaves on the feed: when it started and why, how it ended and the commits it made.

use groove_agent_service::runs::Run;
use groove_types::{Action, TimelineKind};

use super::run::{Ending, routine};
use crate::{AppState, Services, Spawner, Told};

/// `fix-mr · CI failed on main`, on the session it runs on.
pub(super) fn ran(state: &AppState, services: &Services, spawner: &dyn Spawner, run: &Run) {
    let by = match (run.about.is_empty(), run.trigger) {
        (false, _) => run.about.clone(),
        (true, Some(trigger)) => trigger.name().to_string(),
        (true, None) => "its button".to_string(),
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
    let how = match ending {
        Ending::Done => match commits(state, run) {
            0 => "done".to_string(),
            1 => "done · 1 commit".to_string(),
            n => format!("done · {n} commits"),
        },
        Ending::Gone => "its agent stopped".to_string(),
        Ending::Stalled => "its agent never started".to_string(),
    };
    let subject = format!("{} · {how}", name(state, run));
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

/// The commits its session logged since its words reached the agent.
fn commits(state: &AppState, run: &Run) -> usize {
    let Some(sent) = run.sent_at else {
        return 0;
    };
    let feed = state.session.feed.iter();
    let made = |one: &&groove_types::TimelineEvent| {
        one.session == run.session && one.kind == TimelineKind::Commit && one.at >= sent
    };
    feed.filter(made).count()
}

fn name(state: &AppState, run: &Run) -> String {
    routine(state, &run.routine).map_or_else(|| run.routine.clone(), |one| one.name.clone())
}
