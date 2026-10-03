//! A run's turn on its agent: started within the cap, over once its agent stops.

use groove_types::{AgentStatus, RoutineKind, SessionId, Timestamp};

use super::{Run, Runs};

/// How long a run's agent may take to start working before the run is given up.
const STARTS_WITHIN: i64 = 120;

/// How a run came to an end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ending {
    /// Its agent worked, then stopped.
    Done,
    /// Its agent exited or failed.
    Gone,
    /// Its agent never started working.
    Stalled,
}

impl Runs {
    /// The runs waiting their turn that the cap and their agents allow now, moved to running.
    pub fn due(
        &mut self,
        cap: usize,
        kind_of: impl Fn(&str) -> Option<RoutineKind>,
        status_of: impl Fn(&SessionId) -> Option<AgentStatus>,
        now: Timestamp,
    ) -> Vec<Run> {
        let (mut kept, mut started) = (std::collections::VecDeque::new(), Vec::new());
        while let Some(run) = self.waiting.pop_front() {
            let standalone = kind_of(&run.routine) == Some(RoutineKind::Standalone);
            let free = !matches!(
                status_of(&run.session),
                Some(AgentStatus::Working | AgentStatus::Asking)
            );
            let ready = !self.busy(&run.session) && (standalone || free);
            match self.running.len() < cap && ready {
                true => started.push(self.start_now(run, now)),
                false => kept.push_back(run),
            }
        }
        self.waiting = kept;
        started
    }

    /// The run on its agent now, whatever the cap.
    pub fn start_now(&mut self, mut run: Run, now: Timestamp) -> Run {
        run.sent_at = Some(now);
        self.running.push(run.clone());
        run
    }

    /// The runs whose agent worked and stopped, or never started, or went, taken off; how each ended.
    pub fn finished(
        &mut self,
        status_of: impl Fn(&SessionId) -> Option<AgentStatus>,
        now: Timestamp,
    ) -> Vec<(Run, Ending)> {
        let mut ended = Vec::new();
        self.running.retain_mut(|run| {
            let status = status_of(&run.session);
            run.went |= status == Some(AgentStatus::Working);
            let waited = now.seconds() - run.sent_at.unwrap_or(now).seconds();
            let ending = match status {
                Some(AgentStatus::Exited { .. } | AgentStatus::Error { .. }) => Some(Ending::Gone),
                None | Some(AgentStatus::Done { .. } | AgentStatus::Idle) if run.went => {
                    Some(Ending::Done)
                }
                _ if !run.went && waited > STARTS_WITHIN => Some(Ending::Stalled),
                _ => None,
            };
            if let Some(ending) = ending {
                ended.push((run.clone(), ending));
            }
            ending.is_none()
        });
        ended
    }
}
