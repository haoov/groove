//! The rows in the order of one column: a number or an age by its value, anything else as text.

use groove_types::{ObjectRow, WatchKey};
use groove_ui_kit::widgets::Sorted;

use super::plan::Plan;

/// The rows ordered by the column `sort` names; where it stands, for the header's mark.
pub(super) fn sorted(
    rows: &mut [(&WatchKey, &ObjectRow)],
    plan: &Plan,
    sort: Option<&(String, bool)>,
) -> Option<Sorted> {
    let (label, descending) = sort?;
    let column = plan.labels.iter().position(|one| one == label)?;
    let named = usize::from(plan.clusters) + usize::from(plan.namespaces);
    let text = |(key, row): &(&WatchKey, &ObjectRow)| -> String {
        match (column, plan.clusters) {
            (0, true) => key.context.clone(),
            (at, _) if at < named => row.namespace.clone().unwrap_or_default(),
            (at, _) => {
                let cell = plan.shown[at - named];
                row.cells.get(cell).cloned().unwrap_or_default()
            }
        }
    };
    rows.sort_by(|a, b| {
        let order = groove_types::compare_cells(&text(a), &text(b));
        if *descending { order.reverse() } else { order }
    });
    Some(Sorted {
        column,
        descending: *descending,
    })
}
