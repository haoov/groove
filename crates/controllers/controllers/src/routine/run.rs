//! A run on its agent: started within the cap, its words sent, over once its agent stops.

use groove_agent_service::runs::Run;
use groove_types::{AgentStatus, Routine, RoutineKind, Timestamp};

use crate::session::FIRST_SIZE;
use crate::{AppState, Spawner, agent};

/// How long a run's agent may take to start working before the run is given up.
const STARTS_WITHIN: i64 = 120;

/// How a run came to an end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Ending {
    /// Its agent worked, then stopped.
    Done,
    /// Its agent exited or failed.
    Gone,
    /// Its agent never started working.
    Stalled,
}

/// The runs waiting their turn started, as many as the cap leaves room for; those it started.
pub(super) fn pump(state: &mut AppState, spawner: &dyn Spawner) -> Vec<Run> {
    let cap = state
        .config
        .config
        .as_ref()
        .map_or(5, |one| one.preferences.routine_cap) as usize;
    let (mut kept, mut started) = (std::collections::VecDeque::new(), Vec::new());
    while let Some(run) = state.agent.runs.waiting.pop_front() {
        let room = state.agent.runs.running.len() < cap;
        match room && ready(state, &run) {
            true => started.extend(start(state, spawner, run)),
            false => kept.push_back(run),
        }
    }
    state.agent.runs.waiting = kept;
    started
}

/// A standalone run takes its agent afresh; a bound one needs its agent free of other work.
fn ready(state: &AppState, run: &Run) -> bool {
    if state.agent.runs.busy(&run.session) {
        return false;
    }
    let standalone =
        routine(state, &run.routine).is_some_and(|one| one.kind == RoutineKind::Standalone);
    let status = state.agent.activity(&run.session).map(|one| &one.status);
    standalone || !matches!(status, Some(AgentStatus::Working | AgentStatus::Asking))
}

/// The run on its agent now: a fresh agent asked it, or the words typed into the one waiting.
pub(super) fn start(state: &mut AppState, spawner: &dyn Spawner, mut run: Run) -> Option<Run> {
    let routine = routine(state, &run.routine).cloned()?;
    let words = prompt(&routine, &run);
    let id = run.session.clone();
    let alive = state.agent.terminal(&id).is_some_and(|_| {
        !matches!(
            state.agent.activity(&id).map(|one| &one.status),
            Some(AgentStatus::Exited { .. })
        )
    });
    match (routine.kind, alive) {
        (RoutineKind::Bound, true) => agent::skills::typed(state, spawner, &id, &words),
        (RoutineKind::Standalone, _) => {
            agent::end(state, &id);
            if let Err(e) = groove_agent_service::fresh(&agent::launch_dir(state), id.as_str()) {
                state.failed(groove_types::Error::new(
                    groove_types::ErrorKind::Io,
                    e.to_string(),
                ));
            }
            agent::asking(state, spawner, id, FIRST_SIZE, Some(words));
        }
        (RoutineKind::Bound, false) => agent::asking(state, spawner, id, FIRST_SIZE, Some(words)),
        (RoutineKind::Action, _) => return None,
    }
    run.sent_at = Some(Timestamp::now());
    state.agent.runs.running.push(run.clone());
    Some(run)
}

/// The runs whose agent worked and stopped, or never started, or went, taken off; how each ended.
pub(super) fn finished(state: &mut AppState, now: Timestamp) -> Vec<(Run, Ending)> {
    let mut running = std::mem::take(&mut state.agent.runs.running);
    for run in &mut running {
        let status = state
            .agent
            .activity(&run.session)
            .map(|one| one.status.clone());
        if status == Some(AgentStatus::Working) {
            run.went = true;
        }
    }
    let mut ended = Vec::new();
    running.retain(|run| {
        let status = state.agent.activity(&run.session).map(|one| &one.status);
        let sent = run.sent_at.unwrap_or(now);
        let ending = match status {
            Some(AgentStatus::Exited { .. } | AgentStatus::Error { .. }) => Some(Ending::Gone),
            None | Some(AgentStatus::Done { .. } | AgentStatus::Idle) if run.went => {
                Some(Ending::Done)
            }
            _ if !run.went && now.seconds() - sent.seconds() > STARTS_WITHIN => {
                Some(Ending::Stalled)
            }
            _ => None,
        };
        if let Some(ending) = ending {
            ended.push((run.clone(), ending));
        }
        ending.is_none()
    });
    state.agent.runs.running = running;
    ended
}

pub(super) fn routine<'a>(state: &'a AppState, id: &str) -> Option<&'a Routine> {
    let listed = state.agent.routines.iter().find(|one| one.id == id)?;
    listed.read.as_ref().ok()
}

/// What the agent is asked: its one skill alone, or the routine's words with its skills named.
pub(super) fn prompt(routine: &Routine, run: &Run) -> String {
    if routine.words.is_empty()
        && let [skill] = routine.skills.as_slice()
    {
        return format!("/{skill} {}", run.about).trim_end().to_string();
    }
    let by = run.trigger.map_or("its button", |one| one.name());
    let mut said = vec![format!("Routine `{}`, started by {by}.", routine.name)];
    if !run.about.is_empty() {
        said.push(format!("It is about: {}.", run.about));
    }
    if !routine.skills.is_empty() {
        let skills: Vec<String> = routine.skills.iter().map(|one| format!("/{one}")).collect();
        said.push(format!("Use the skills {}.", skills.join(", ")));
    }
    said.push(String::new());
    said.push(routine.words.clone());
    said.join("\n")
}
