//! The rows in the order of one column: a number or an age by its value, anything else as text.
//! The order is kept between frames while the rows, the search and the sort stay the same.

use std::cell::RefCell;
use std::cmp::Ordering;
use std::rc::Rc;

use groove_controllers::AppState;
use groove_types::{ObjectRow, WatchKey};
use groove_ui_kit::widgets::Sorted;

use super::plan::Plan;

/// What an order was built for: each key at its revision, the search, the sort.
#[derive(Debug, Clone, PartialEq)]
struct Stamp {
    keys: Vec<(WatchKey, u64)>,
    search: String,
    sort: Option<(String, bool)>,
}

/// Each row by its key's place and its own.
type Order = Rc<Vec<(usize, usize)>>;

/// The last order built.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Kept {
    kept: RefCell<Option<(Stamp, Order)>>,
}

impl Kept {
    /// The order for this frame, built again only when what it was built for moved.
    pub(super) fn of(
        &self,
        app: &AppState,
        (keys, search, sort): (&[WatchKey], &str, Option<&(String, bool)>),
        build: impl FnOnce() -> Vec<(usize, usize)>,
    ) -> Order {
        let revision =
            |key: &WatchKey| app.cluster.store.watched(key).map_or(0, |one| one.revision);
        let stamp = Stamp {
            keys: keys
                .iter()
                .map(|key| (key.clone(), revision(key)))
                .collect(),
            search: search.to_string(),
            sort: sort.cloned(),
        };
        let mut kept = self.kept.borrow_mut();
        if let Some((at, order)) = kept.as_ref()
            && *at == stamp
        {
            return order.clone();
        }
        let order = Rc::new(build());
        *kept = Some((stamp, order.clone()));
        order
    }
}

/// Where the header's sort mark stands, for the column `sort` names.
pub(super) fn mark(plan: &Plan, sort: Option<&(String, bool)>) -> Option<Sorted> {
    let (label, descending) = sort?;
    let column = plan.labels.iter().position(|one| one == label)?;
    Some(Sorted {
        column,
        descending: *descending,
    })
}

/// The rows ordered by the column `sort` names, each cell read once; `at` is each row, aligned.
pub(super) fn sorted<'a>(
    rows: Vec<(usize, usize)>,
    at: &[(&'a WatchKey, &'a ObjectRow)],
    plan: &Plan,
    sort: Option<&(String, bool)>,
) -> Vec<(usize, usize)> {
    let Some(Sorted { column, descending }) = mark(plan, sort) else {
        return rows;
    };
    let named = usize::from(plan.clusters) + usize::from(plan.namespaces);
    let text = |(key, row): &(&'a WatchKey, &'a ObjectRow)| -> &'a str {
        match (column, plan.clusters) {
            (0, true) => key.context.as_str(),
            (at, _) if at < named => row.namespace.as_deref().unwrap_or_default(),
            (at, _) => {
                let cell = plan.shown[at - named];
                row.cells.get(cell).map_or("", String::as_str)
            }
        }
    };
    let texts: Vec<&str> = at.iter().map(text).collect();
    let values: Vec<Option<f64>> = texts
        .iter()
        .map(|one| groove_types::cell_value(one))
        .collect();
    let mut places: Vec<usize> = (0..rows.len()).collect();
    places.sort_by(|a, b| {
        let order = match (values[*a], values[*b]) {
            (Some(x), Some(y)) => x.partial_cmp(&y).unwrap_or(Ordering::Equal),
            _ => texts[*a].cmp(texts[*b]),
        };
        if descending { order.reverse() } else { order }
    });
    places.into_iter().map(|at| rows[at]).collect()
}
