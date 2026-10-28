//! The plan: the tasks waiting, in the order the user gave them.

#[cfg(test)]
mod tests;

use groove_plan::Placed;
use groove_types::{ExternalId, Task};

/// One task in the plan.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Planned<'a> {
    pub task: &'a Task,
}

/// The tasks in the user's order: the placed ones, then the rest as the source answered.
pub fn ordered<'a>(order: &[Placed], tasks: &[&'a Task]) -> Vec<Planned<'a>> {
    let mut left: Vec<&'a Task> = tasks.to_vec();
    let mut planned = Vec::with_capacity(left.len());
    for placed in order {
        let Some(at) = left
            .iter()
            .position(|task| task.external_id == placed.external_id)
        else {
            continue;
        };
        planned.push(Planned {
            task: left.remove(at),
        });
    }
    planned.extend(left.into_iter().map(|task| Planned { task }));
    planned
}

/// The order after `id` moves above `before`, or to the end.
pub fn moved(shown: &[Planned<'_>], id: &ExternalId, before: Option<&ExternalId>) -> Vec<Placed> {
    let mut order: Vec<Placed> = shown
        .iter()
        .filter(|one| one.task.external_id != *id)
        .map(|one| Placed::new(one.task.external_id.clone()))
        .collect();
    let at = before
        .and_then(|before| order.iter().position(|one| one.external_id == *before))
        .unwrap_or(order.len());
    order.insert(at, Placed::new(id.clone()));
    order
}

impl crate::State {
    /// Each task's short id and status, by its id at the source.
    pub fn statuses(
        &self,
    ) -> std::collections::BTreeMap<groove_types::ExternalId, (String, String)> {
        let each = self.tasks.iter().map(|one| {
            (
                one.external_id.clone(),
                (one.short_id.clone(), one.status.clone()),
            )
        });
        each.collect()
    }

    /// Up Next's tasks no session works that must start today, in the user's order, `cap` at most.
    pub fn due_today(
        &self,
        worked: &[groove_types::ExternalId],
        today: groove_types::Day,
        cap: usize,
    ) -> Vec<String> {
        let waiting = self.waiting(worked);
        let planned = self.planned(&waiting).into_iter();
        let due = planned.filter(|one| groove_types::starts_today(one.task, today));
        due.map(|one| one.task.short_id.clone()).take(cap).collect()
    }
}
