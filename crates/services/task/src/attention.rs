//! What needs the user, task by task.

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use groove_types::{Attention, Day, ExternalId, MrFacts, Task, Thresholds, Timestamp, attention};

use crate::State;

impl State {
    /// Starts the source left out filled from the first session, then what needs the user.
    pub fn reread(
        &mut self,
        started: impl IntoIterator<Item = (ExternalId, Day)>,
        mrs: impl IntoIterator<Item = (ExternalId, MrFacts)>,
        now: Timestamp,
        thresholds: &Thresholds,
    ) {
        let mut first: BTreeMap<ExternalId, Day> = BTreeMap::new();
        for (task, day) in started {
            first
                .entry(task)
                .and_modify(|held| *held = (*held).min(day))
                .or_insert(day);
        }
        for task in &mut self.tasks {
            task.dates.start = task
                .dates
                .start
                .or_else(|| first.get(&task.external_id).copied());
        }
        let mut facts: BTreeMap<ExternalId, MrFacts> = BTreeMap::new();
        for (task, one) in mrs {
            facts
                .entry(task)
                .and_modify(|held| *held = held.and(one))
                .or_insert(one);
        }
        self.attention = folded(&self.tasks, &facts, now, thresholds);
    }
}

/// Every task's reasons, the ones with none left out.
pub fn folded(
    tasks: &[Task],
    facts: &BTreeMap<ExternalId, MrFacts>,
    now: Timestamp,
    thresholds: &Thresholds,
) -> BTreeMap<ExternalId, Vec<Attention>> {
    let mut out = BTreeMap::new();
    for task in tasks {
        let held = facts.get(&task.external_id).copied().unwrap_or_default();
        let reasons = attention(&held, &task.dates, now, thresholds);
        if !reasons.is_empty() {
            out.insert(task.external_id.clone(), reasons);
        }
    }
    out
}
