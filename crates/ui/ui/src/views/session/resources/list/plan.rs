//! What the list shows, worked out from its rows: the columns, their labels and widths, what each says.

use groove_controllers::AppState;
use groove_types::{ObjectRow, WatchKey};
use groove_ui_kit::base::style::Role;

/// How many rows a column's width is fitted over.
const FITTED: usize = 200;

/// What the list shows: the Table's own default columns, behind a cluster column and a
/// namespace column where those tell rows apart; each with its label and its widest sample.
pub(super) struct Plan {
    pub(super) shown: Vec<usize>,
    /// What each shown column says: a status, a ready count, or neither.
    pub(super) reads: Vec<Reads>,
    pub(super) clusters: bool,
    pub(super) namespaces: bool,
    pub(super) labels: Vec<String>,
    pub(super) samples: Vec<String>,
}

pub(super) fn plan(app: &AppState, keys: &[WatchKey], rows: &[(&WatchKey, &ObjectRow)]) -> Plan {
    let columns = table_columns(app, keys);
    let shown: Vec<usize> = (0..columns.len())
        .filter(|at| columns[*at].priority == 0)
        .collect();
    let clusters = distinct(keys.iter().map(|key| key.context.as_str())) > 1;
    let namespaced = keys.iter().any(|key| key.kind.namespaced);
    let in_rows = rows.iter().filter_map(|(_, row)| row.namespace.as_deref());
    let namespaces = namespaced && distinct(in_rows) > 1;
    let widest = |texts: &mut dyn Iterator<Item = &str>| {
        let longest = texts.take(FITTED).max_by_key(|one| one.chars().count());
        longest.unwrap_or_default().to_string()
    };
    let cell = |at: usize| {
        widest(
            &mut rows
                .iter()
                .map(|(_, row)| row.cells.get(at).map_or("", String::as_str)),
        )
    };
    let mut labels = Vec::new();
    let mut samples = Vec::new();
    if clusters {
        labels.push("CLUSTER".to_string());
        samples.push(widest(&mut keys.iter().map(|key| key.context.as_str())));
    }
    if namespaces {
        labels.push("NAMESPACE".to_string());
        samples.push(widest(
            &mut rows
                .iter()
                .map(|(_, row)| row.namespace.as_deref().unwrap_or_default()),
        ));
    }
    let mut reads = Vec::new();
    for at in &shown {
        let label = columns[*at].name.to_uppercase();
        reads.push(Reads::of(&label.to_lowercase()));
        labels.push(label);
        samples.push(cell(*at));
    }
    Plan {
        shown,
        reads,
        clusters,
        namespaces,
        labels,
        samples,
    }
}

/// The columns of the first key whose list carries them.
fn table_columns<'a>(app: &'a AppState, keys: &[WatchKey]) -> &'a [groove_types::TableColumn] {
    let watched = keys.iter().filter_map(|key| app.cluster.store.watched(key));
    let carried = watched
        .map(|one| one.columns.as_slice())
        .find(|one| !one.is_empty());
    carried.unwrap_or_default()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Reads {
    Plain,
    Status,
    Ready,
}

pub(super) fn colour(health: groove_types::Health) -> Role {
    match health {
        groove_types::Health::Good => Role::Ok,
        groove_types::Health::Waiting => Role::Warn,
        groove_types::Health::Failing => Role::Bad,
        groove_types::Health::Done => Role::Ghost,
    }
}

impl Reads {
    /// What a column says, by its name.
    fn of(label: &str) -> Self {
        match label {
            "status" | "phase" => Reads::Status,
            "ready" => Reads::Ready,
            _ => Reads::Plain,
        }
    }
}

fn distinct<'a>(items: impl Iterator<Item = &'a str>) -> usize {
    items.collect::<std::collections::BTreeSet<_>>().len()
}
