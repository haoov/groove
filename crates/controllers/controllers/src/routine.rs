//! The routines at work: each look at the state turns what changed into runs, and runs them.

mod action;
mod run;
mod seen;
mod trail;

use groove_agent_service::runs::{Fired, Run, target};
use groove_types::{Action, Error, RoutineKind, Timestamp, Trigger};

use crate::{AppState, Services, Spawner};

/// What changed since the last look, as runs: queued by the rules, started within the cap.
pub fn watch(state: &mut AppState, services: &Services, spawner: &dyn Spawner) {
    let before = state.agent.runs.seen.take();
    let now = seen::look(state, before.as_ref());
    if let Some(selected) = &now.selected {
        state.agent.runs.seen_session(selected);
    }
    let fired = before
        .as_ref()
        .map(|one| one.fired(&now))
        .unwrap_or_default();
    state.agent.runs.seen = Some(now);
    let fired = fired.into_iter().filter(|one| !ran_itself(state, one));
    let mut fired: Vec<Fired> = fired.collect();
    fired.extend(dawn(state));
    for one in fire(state, &fired) {
        action::act(state, services, spawner, one);
    }
    turn(state, services, spawner);
}

/// The runs that ended, written down; those the cap lets start, sent.
fn turn(state: &mut AppState, services: &Services, spawner: &dyn Spawner) {
    let now = Timestamp::now();
    for one in &state.agent.runs_ended(now) {
        trail::ended(state, services, spawner, one);
    }
    let cap = state.config.preferences().routine_cap as usize;
    for one in &state.agent.runs_due(cap, now) {
        run::send(state, spawner, one);
        trail::started(state, services, spawner, one);
    }
}

/// Each event weighed by the rules, unless the routines are paused; the actions they ask.
fn fire(state: &mut AppState, fired: &[Fired]) -> Vec<Action> {
    if state.config.preferences().routines_paused {
        return Vec::new();
    }
    let (routines, held) = (state.agent.readable_routines(), state.config.routines());
    let sessions = seen::sessions(state);
    let runs = &mut state.agent.runs;
    let each = fired
        .iter()
        .map(|one| runs.fire(one, &routines, &held, &sessions));
    each.collect::<Vec<_>>().concat()
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

/// The daily trigger, on the day's first look once the rail, the routines and the tasks are back.
fn dawn(state: &mut AppState) -> Option<Fired> {
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
        return None;
    }
    state.agent.runs.dawned = true;
    let dir = state.env.data_dir.join("routines");
    let today = Timestamp::now().day().to_string();
    match groove_agent_service::routines::new_day(&dir, &today) {
        Ok(true) => Some(Fired {
            trigger: Trigger::Daily,
            session: None,
            about: String::new(),
        }),
        Ok(false) => None,
        Err(e) => {
            state.failed(e);
            None
        }
    }
}

/// A standalone routine's button runs it now in its session, an action's does it; whatever the rules.
pub fn button(state: &mut AppState, services: &Services, spawner: &dyn Spawner, id: &str) {
    let Some(routine) = state.agent.routine(id).cloned() else {
        let why = format!("`{id}` is no routine Groove can run");
        return state.failed(Error::invalid(why));
    };
    let session = target(&routine, None, &seen::sessions(state));
    let why = match (routine.kind, routine.action, &session) {
        (_, Some(one), _) => return action::act(state, services, spawner, one),
        (RoutineKind::Standalone, _, Some(session)) if state.agent.runs.holds(id, session) => {
            "it runs there already"
        }
        (RoutineKind::Standalone, _, Some(_)) => "",
        (RoutineKind::Standalone, _, None) => "switch it on first: its session holds its agent",
        (RoutineKind::Bound, ..) => "it is bound: its events run it",
        (RoutineKind::Action, ..) => "it does nothing Groove knows",
    };
    let Some(session) = session.filter(|_| why.is_empty()) else {
        return state.failed(Error::invalid(format!("`{id}` cannot run: {why}")));
    };
    let run = Run::new(&routine.id, session, None, "");
    let started = state.agent.runs.start_now(run, Timestamp::now());
    run::send(state, spawner, &started);
    trail::started(state, services, spawner, &started);
}
