//! The plan: the tasks waiting, in the order the user gave them.

#[cfg(test)]
mod tests;

use groove_plan::Placed;
use groove_types::{ExternalId, Task};

/// One task in the plan, and which side of the divider it stands.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Planned<'a> {
    pub task: &'a Task,
    pub later: bool,
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
            later: placed.later,
        });
    }
    let (now, later): (Vec<Planned<'_>>, Vec<Planned<'_>>) =
        planned.into_iter().partition(|one| !one.later);
    let rest = left.into_iter().map(|task| Planned { task, later: false });
    now.into_iter().chain(rest).chain(later).collect()
}

/// The order after `id` moves above `before`, or to the end of its side.
pub fn moved(
    shown: &[Planned<'_>],
    id: &ExternalId,
    before: Option<&ExternalId>,
    later: bool,
) -> Vec<Placed> {
    let mut order: Vec<Placed> = shown
        .iter()
        .filter(|one| one.task.external_id != *id)
        .map(|one| Placed {
            external_id: one.task.external_id.clone(),
            later: one.later,
        })
        .collect();
    let moving = Placed {
        external_id: id.clone(),
        later,
    };
    let at = match before {
        Some(before) => order
            .iter()
            .position(|one| one.external_id == *before)
            .unwrap_or(order.len()),
        None => ends(&order, later),
    };
    order.insert(at, moving);
    order
}

/// Where a side of the divider ends.
fn ends(order: &[Placed], later: bool) -> usize {
    match later {
        true => order.len(),
        false => order
            .iter()
            .position(|one| one.later)
            .unwrap_or(order.len()),
    }
}
