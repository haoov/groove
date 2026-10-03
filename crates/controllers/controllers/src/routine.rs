//! The routines at work: each look at the state turns what changed into runs, and runs them.

mod rules;
mod run;
mod seen;

use groove_agent_service::runs::{Fired, Run};
use groove_types::{Error, RoutineKind, Timestamp, Trigger};

use crate::{AppState, Services, Spawner};

/// What changed since the last look, as runs: queued by the rules, started within the cap.
pub fn watch(state: &mut AppState, _: &Services, spawner: &dyn Spawner) {
    let before = state.agent.runs.seen.take();
    let now = seen::look(state, before.as_ref());
    if let Some(selected) = &now.selected {
        state.agent.runs.looked(selected);
    }
    let fired = match &before {
        Some(before) => seen::fired(state, before, &now),
        None => Vec::new(),
    };
    state.agent.runs.seen = Some(now);
    let fired: Vec<Fired> = fired
        .into_iter()
        .filter(|one| !ran_itself(state, one))
        .collect();
    for one in &fired {
        rules::queue(state, one);
    }
    dawn(state);
    run::finished(state, Timestamp::now());
    run::pump(state, spawner);
}

/// An agent that finished a run does not start another routine by finishing.
fn ran_itself(state: &AppState, fired: &Fired) -> bool {
    let finished = fired.trigger == Trigger::AgentFinished;
    finished
        && fired
            .session
            .as_ref()
            .is_some_and(|one| state.agent.runs.busy(one))
}

/// The day's first look, once the rail and the routines are back, fires the daily trigger.
fn dawn(state: &mut AppState) {
    let back = state.session.restored && state.agent.listed;
    if state.agent.runs.dawned || !back {
        return;
    }
    state.agent.runs.dawned = true;
    let path = state.env.data_dir.join("routines").join("day");
    let today = Timestamp::now().day().to_string();
    if std::fs::read_to_string(&path).is_ok_and(|day| day.trim() == today) {
        return;
    }
    let written = std::fs::create_dir_all(state.env.data_dir.join("routines"))
        .and_then(|()| std::fs::write(&path, &today));
    if let Err(e) = written {
        state.failed(Error::new(groove_types::ErrorKind::Io, e.to_string()));
    }
    let daily = Fired {
        trigger: Trigger::Daily,
        session: None,
        about: String::new(),
    };
    rules::queue(state, &daily);
}

/// The routine's button: it runs now, on its own session or the selected one, whatever the rules.
pub fn button(state: &mut AppState, spawner: &dyn Spawner, id: &str) {
    let listed = state.agent.routines.iter().find(|one| one.id == id);
    let Some(routine) = listed.and_then(|one| one.read.as_ref().ok()).cloned() else {
        return state.failed(Error::invalid(format!(
            "`{id}` is no routine Groove can run"
        )));
    };
    let selected = state.session.selected.clone();
    let Some(session) = rules::target(state, &routine, selected.as_ref()) else {
        let why = match routine.kind {
            RoutineKind::Bound => "select the session it should run on first",
            RoutineKind::Standalone => "switch it on first: its session holds its agent",
        };
        return state.failed(Error::invalid(format!("`{id}` cannot run: {why}")));
    };
    if state.agent.runs.holds(id, &session) {
        return state.failed(Error::invalid(format!("`{id}` runs there already")));
    }
    let run = Run {
        routine: routine.id,
        session,
        trigger: None,
        about: String::new(),
        sent_at: None,
        went: false,
    };
    run::start(state, spawner, run);
}
