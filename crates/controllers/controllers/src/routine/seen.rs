//! What a look at the state sees, and the events between two looks.

use groove_agent_service::runs::{Fired, Seen};
use groove_types::{AgentStatus, CiState, ReviewState, SessionId, Trigger};

use crate::AppState;

/// The state as the triggers read it; `before` carries what only a change can tell.
pub(super) fn look(state: &AppState, before: Option<&Seen>) -> Seen {
    let mut seen = Seen {
        selected: state.session.selected.clone(),
        reading: state.task.reading,
        tasks_read: before.is_some_and(|one| one.tasks_read || one.reading && !state.task.reading),
        ..Seen::default()
    };
    let worktrees = state.session.open.iter().flat_map(|open| &open.worktrees);
    for worktree in worktrees {
        let Some(held) = state.delivery.held(&worktree.id) else {
            continue;
        };
        if let Some(ci) = held.ci() {
            seen.ci.insert(worktree.id.clone(), ci);
        }
        if let Some(read) = &held.read {
            let asked = read.details.changes_requested();
            seen.changes.insert(worktree.id.clone(), asked);
        }
    }
    seen.asked = state.delivery.reviews_read.then(|| {
        let waiting = |one: &&groove_types::ReviewMr| {
            matches!(one.review, None | Some(ReviewState::Requested))
        };
        let reviews = state.delivery.reviews.iter().filter(waiting);
        reviews.map(|one| one.session_id()).collect()
    });
    seen.working = state
        .agent
        .agents
        .iter()
        .filter(|(_, agent)| agent.activity.status == AgentStatus::Working)
        .map(|(id, _)| id.clone())
        .collect();
    seen.tasks = state
        .task
        .tasks
        .iter()
        .map(|one| {
            (
                one.external_id.clone(),
                (one.short_id.clone(), one.status.clone()),
            )
        })
        .collect();
    seen
}

/// Every event between two looks, by the session it is about.
pub(super) fn fired(state: &AppState, before: &Seen, now: &Seen) -> Vec<Fired> {
    let mut out = delivered(state, before, now);
    for session in before.working.difference(&now.working) {
        let status = state.agent.activity(session).map(|one| &one.status);
        if matches!(status, Some(AgentStatus::Done { .. })) {
            out.push(event(Trigger::AgentFinished, Some(session.clone()), ""));
        }
    }
    out.extend(tasks(before, now));
    out
}

/// What the forges said: a run gone red, changes asked for, a review asked of the user.
fn delivered(state: &AppState, before: &Seen, now: &Seen) -> Vec<Fired> {
    let mut out = Vec::new();
    for (worktree, ci) in &now.ci {
        let was = before.ci.get(worktree);
        let red = *ci == CiState::Failed && was.is_some_and(|one| *one != CiState::Failed);
        if red && let Some((session, branch)) = owner(state, worktree) {
            let said = format!("CI failed on {branch}");
            out.push(event(Trigger::CiFailed, Some(session), &said));
        }
    }
    for (worktree, asked) in &now.changes {
        let new = *asked && before.changes.get(worktree) == Some(&false);
        if new && let Some((session, branch)) = owner(state, worktree) {
            let said = format!("changes requested on {branch}");
            out.push(event(Trigger::ChangesRequested, Some(session), &said));
        }
    }
    if let (Some(was), Some(is)) = (&before.asked, &now.asked) {
        for review in is.difference(was) {
            out.push(event(
                Trigger::ReviewAsked,
                Some(SessionId::new(review)),
                "",
            ));
        }
    }
    out
}

/// The tasks read again, and each one whose status moved since the last read.
fn tasks(before: &Seen, now: &Seen) -> Vec<Fired> {
    let mut out = Vec::new();
    if before.reading && !now.reading {
        out.push(event(Trigger::TasksRead, None, ""));
    }
    if !before.tasks_read {
        return out;
    }
    for (task, (short, status)) in &now.tasks {
        let was = before.tasks.get(task).map(|(_, one)| one);
        if was.is_some_and(|one| one != status) {
            let said = format!("{short} moved to {status}");
            out.push(event(Trigger::TaskMoved, None, &said));
        }
    }
    out
}

fn event(trigger: Trigger, session: Option<SessionId>, about: &str) -> Fired {
    Fired {
        trigger,
        session,
        about: about.to_string(),
    }
}

/// The session a worktree belongs to, and its branch.
fn owner(state: &AppState, worktree: &groove_types::WorktreeId) -> Option<(SessionId, String)> {
    state.session.open.iter().find_map(|open| {
        let one = open.worktrees.iter().find(|one| &one.id == worktree)?;
        Some((open.session.id.clone(), one.branch.clone()))
    })
}
