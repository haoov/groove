//! What needs the user, task by task.

use std::collections::BTreeMap;

use groove_types::{Attention, ExternalId, MrFacts, Task, Thresholds, Timestamp, attention};

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
