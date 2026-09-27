//! The clock that credits the task being worked, and what it owes the ledger.

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use groove_types::{ExternalId, Timestamp};

/// The longest run the clock trusts: a gap wider than this was not work.
pub const IDLE: i64 = 120;

/// Whether the window's task counts as worked: in focus, and the user or its agent busy.
pub fn working(focused: bool, acted_at: Timestamp, busy: bool, now: Timestamp) -> bool {
    focused && (busy || now.seconds() - acted_at.seconds() <= IDLE)
}

/// What the timer stands on and what it has measured since the ledger last took it.
#[derive(Debug, Default)]
pub struct Timer {
    on: Option<(ExternalId, Timestamp)>,
    owed: BTreeMap<ExternalId, i64>,
    wrote_at: Option<Timestamp>,
}

impl Timer {
    /// The task the clock now runs on, closing the run it stood on before.
    pub fn on(&mut self, task: Option<ExternalId>, now: Timestamp) {
        let same = self
            .on
            .as_ref()
            .is_some_and(|(held, _)| Some(held) == task.as_ref());
        if same {
            return;
        }
        if let Some((held, since)) = self.on.take() {
            let run = (now.seconds() - since.seconds()).min(IDLE);
            if run > 0 {
                *self.owed.entry(held).or_default() += run;
            }
        }
        self.on = task.map(|task| (task, now));
    }

    /// Whether the ledger has waited long enough for what the clock owes it.
    pub fn due(&self, now: Timestamp, every: i64) -> bool {
        let waited = self
            .wrote_at
            .is_none_or(|at| now.seconds() - at.seconds() >= every);
        waited && (!self.owed.is_empty() || self.standing(now) > 0)
    }

    /// What the ledger takes: every run closed, the one still open counted to now.
    pub fn taken(&mut self, now: Timestamp) -> Vec<(ExternalId, i64)> {
        if let Some((held, _)) = self.on.clone() {
            self.on(None, now);
            self.on = Some((held, now));
        }
        self.wrote_at = Some(now);
        std::mem::take(&mut self.owed).into_iter().collect()
    }

    /// The seconds the open run has measured so far.
    fn standing(&self, now: Timestamp) -> i64 {
        self.on
            .as_ref()
            .map_or(0, |(_, since)| now.seconds() - since.seconds())
    }
}
