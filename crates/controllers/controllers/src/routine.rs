//! The routines at work: each look at the state turns what changed into runs, and runs them.

mod action;
mod rules;
mod run;
mod seen;
mod trail;

use groove_agent_service::runs::{Fired, Run};
use groove_types::{Error, RoutineKind, Timestamp, Trigger};

use crate::{AppState, Services, Spawner};

/// What changed since the last look, as runs: queued by the rules, started within the cap.
pub fn watch(state: &mut AppState, services: &Services, spawner: &dyn Spawner) {
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
    let mut actions = Vec::new();
    for one in &fired {
        actions.extend(rules::queue(state, one));
    }
    actions.extend(dawn(state));
    for one in actions {
        action::act(state, services, spawner, one);
    }
    for ended in run::finished(state, Timestamp::now()) {
        trail::ended(state, services, spawner, &ended);
    }
    for started in run::pump(state, spawner) {
        trail::ran(state, services, spawner, &started);
    }
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

/// The day's first look, once the rail, the routines and the tasks are back, fires the daily trigger.
fn dawn(state: &mut AppState) -> Vec<groove_types::Action> {
    let sourced = !groove_task_service::source_ids(state.config.config.as_ref()).is_empty();
    let read = state
        .agent
        .runs
        .seen
        .as_ref()
        .is_some_and(|one| one.tasks_read);
    let back = state.session.restored && state.agent.listed && (read || !sourced);
    let paused = state.config.preferences().routines_paused;
    if state.agent.runs.dawned || !back || paused {
        return Vec::new();
    }
    state.agent.runs.dawned = true;
    let path = state.env.data_dir.join("routines").join("day");
    let today = Timestamp::now().day().to_string();
    if std::fs::read_to_string(&path).is_ok_and(|day| day.trim() == today) {
        return Vec::new();
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
    rules::queue(state, &daily)
}

/// A standalone routine's button runs it now in its session, an action's does it; whatever the rules.
pub fn button(state: &mut AppState, services: &Services, spawner: &dyn Spawner, id: &str) {
    let listed = state.agent.routines.iter().find(|one| one.id == id);
    let Some(routine) = listed.and_then(|one| one.read.as_ref().ok()).cloned() else {
        return state.failed(Error::invalid(format!(
            "`{id}` is no routine Groove can run"
        )));
    };
    if let Some(one) = routine.action {
        return action::act(state, services, spawner, one);
    }
    if routine.kind == RoutineKind::Bound {
        let why = format!("`{id}` is bound: its events run it");
        return state.failed(Error::invalid(why));
    }
    let selected = state.session.selected.clone();
    let Some(session) = rules::target(state, &routine, selected.as_ref()) else {
        let why = match routine.kind {
            RoutineKind::Bound => "its events run it",
            RoutineKind::Standalone => "switch it on first: its session holds its agent",
            RoutineKind::Action => "it does nothing Groove knows",
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
    if let Some(started) = run::start(state, spawner, run) {
        trail::ran(state, services, spawner, &started);
    }
}
