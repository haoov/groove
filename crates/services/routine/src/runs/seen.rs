//! What a look at the state saw, and the events between two looks.

use std::collections::{BTreeMap, BTreeSet};

use groove_types::{CiState, ExternalId, SessionId, Trigger, WorktreeId};

use super::Fired;

/// What the last look saw, for the next one to tell what changed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Seen {
    pub selected: Option<SessionId>,
    /// The session each worktree of the rail belongs to, and its branch.
    pub owners: BTreeMap<WorktreeId, (SessionId, String)>,
    pub ci: BTreeMap<WorktreeId, CiState>,
    pub changes: BTreeMap<WorktreeId, bool>,
    pub commented: BTreeMap<WorktreeId, bool>,
    /// The review sessions the queue asks of the user, once it is read.
    pub asked: Option<BTreeSet<String>>,
    pub working: BTreeSet<SessionId>,
    /// The agents whose turn ended.
    pub done: BTreeSet<SessionId>,
    /// Each task's short id and status.
    pub tasks: BTreeMap<ExternalId, (String, String)>,
    pub reading: bool,
    /// A read of the tasks has come back since the app started.
    pub tasks_read: bool,
}

impl Seen {
    /// Every event between this look and the one after it.
    pub fn fired(&self, now: &Seen) -> Vec<Fired> {
        let mut out = self.ci_failed(now);
        out.extend(self.reviewed(now));
        let finished = self
            .working
            .difference(&now.working)
            .filter(|one| now.done.contains(*one));
        out.extend(finished.map(|one| event(Trigger::AgentFinished, Some(one.clone()), "")));
        out.extend(self.tasks_moved(now));
        out
    }

    fn ci_failed(&self, now: &Seen) -> Vec<Fired> {
        let red = |(worktree, ci): (&WorktreeId, &CiState)| {
            let was = self.ci.get(worktree);
            let turned = *ci == CiState::Failed && was.is_some_and(|one| *one != CiState::Failed);
            let (session, branch) = now.owners.get(worktree).filter(|_| turned)?;
            let said = format!("CI failed on {branch}");
            Some(event(Trigger::CiFailed, Some(session.clone()), &said))
        };
        now.ci.iter().filter_map(red).collect()
    }

    /// Changes asked for or a review of comments, on a known MR; a review asked of the user.
    fn reviewed(&self, now: &Seen) -> Vec<Fired> {
        let mut out = Vec::new();
        let kinds = [
            (
                &now.changes,
                &self.changes,
                Trigger::ChangesRequested,
                "changes requested",
            ),
            (
                &now.commented,
                &self.commented,
                Trigger::ReviewCommented,
                "a review commented",
            ),
        ];
        for (is, was, trigger, what) in kinds {
            for (worktree, said) in is {
                let new = *said && was.get(worktree) == Some(&false);
                if let Some((session, branch)) = now.owners.get(worktree).filter(|_| new) {
                    let about = format!("{what} on {branch}");
                    out.push(event(trigger, Some(session.clone()), &about));
                }
            }
        }
        if let (Some(was), Some(is)) = (&self.asked, &now.asked) {
            let asked = is.difference(was).map(SessionId::new);
            out.extend(asked.map(|one| event(Trigger::ReviewAsked, Some(one), "")));
        }
        out
    }

    /// The tasks read again, and each one whose status moved since the last read.
    fn tasks_moved(&self, now: &Seen) -> Vec<Fired> {
        let mut out = Vec::new();
        if self.reading && !now.reading {
            out.push(event(Trigger::TasksRead, None, ""));
        }
        if !self.tasks_read {
            return out;
        }
        for (task, (short, status)) in &now.tasks {
            let was = self.tasks.get(task).map(|(_, one)| one);
            if was.is_some_and(|one| one != status) {
                let said = format!("{short} moved to {status}");
                out.push(event(Trigger::TaskMoved, None, &said));
            }
        }
        out
    }
}

fn event(trigger: Trigger, session: Option<SessionId>, about: &str) -> Fired {
    Fired {
        trigger,
        session,
        about: about.to_string(),
    }
}
