//! The rows of every watcher something reads, who reads each, and the kinds of each context.

mod follows;
mod leases;
mod logs;
mod yamls;

pub use follows::{Follow, Follows, Named};
pub use logs::{CAP, Log, Logs, Streaming};
pub use yamls::Yamls;

use std::collections::{BTreeMap, BTreeSet, HashMap};

use groove_objects::{Batch, Delta, Stop};
use groove_types::{KubeKind, ObjectRow, TableColumn, Timestamp, WatchKey};

/// One watcher's rows, by namespace then name, and how it stands.
#[derive(Debug, Default)]
pub struct Watched {
    pub columns: Vec<TableColumn>,
    pub rows: Vec<ObjectRow>,
    /// The first list has landed.
    pub synced: bool,
    /// Why the watcher stands still, until its next batch lands.
    pub failed: Option<String>,
    /// Moves on with every change to the rows.
    pub revision: u64,
}

/// The watchers and the kinds, beside the rest of the `cluster` slice.
#[derive(Debug, Default)]
pub struct Store {
    watched: leases::Leases<WatchKey, Watched>,
    kinds: BTreeMap<String, (Vec<KubeKind>, Timestamp)>,
    discovering: BTreeSet<String>,
    /// The discoveries that failed in a row, by context, while one waits to try again.
    refused: BTreeMap<String, u32>,
    namespaces: BTreeMap<String, Vec<String>>,
    /// When a time cell next reads differently.
    due: Option<Timestamp>,
    pub follows: Follows,
    pub logs: Logs,
}

impl Store {
    pub fn watched(&self, key: &WatchKey) -> Option<&Watched> {
        self.watched.get(key)
    }

    pub fn kinds(&self, context: &str) -> Option<&[KubeKind]> {
        self.kinds.get(context).map(|(kinds, _)| kinds.as_slice())
    }

    /// When the kinds of `context` were last discovered.
    pub fn discovered(&self, context: &str) -> Option<Timestamp> {
        self.kinds.get(context).map(|(_, at)| *at)
    }

    pub fn set_kinds(&mut self, context: &str, kinds: Vec<KubeKind>) {
        self.discovering.remove(context);
        self.refused.remove(context);
        let at = Timestamp::now();
        self.kinds.insert(context.to_string(), (kinds, at));
    }

    /// The namespaces of `context`, as last listed.
    pub fn namespaces(&self, context: &str) -> Option<&[String]> {
        self.namespaces.get(context).map(Vec::as_slice)
    }

    pub fn set_namespaces(&mut self, context: &str, names: Vec<String>) {
        self.namespaces.insert(context.to_string(), names);
    }

    /// Marks a discovery of `context` begun. False while one already runs.
    pub fn begin_discovery(&mut self, context: &str) -> bool {
        self.discovering.insert(context.to_string())
    }

    pub fn discovering(&self, context: &str) -> bool {
        self.discovering.contains(context)
    }

    /// A discovery of `context` failed and stays begun; the failures in a row.
    pub fn refuse_discovery(&mut self, context: &str) -> u32 {
        let failures = self.refused.entry(context.to_string()).or_default();
        *failures += 1;
        *failures
    }

    /// `context` may be discovered again, unless another failure or a sign-in came since `failures`.
    pub fn retry_discovery(&mut self, context: &str, failures: u32) {
        if self.refused.get(context) == Some(&failures) {
            self.discovering.remove(context);
        }
    }

    /// `context` signed in: a discovery waiting on a failure may run now.
    pub fn signed_in(&mut self, context: &str) {
        if self.refused.remove(context).is_some() {
            self.discovering.remove(context);
        }
    }

    /// Every key `reader` reads.
    pub fn read_by<'a>(&'a self, reader: &'a str) -> impl Iterator<Item = &'a WatchKey> {
        self.watched.read_by(reader)
    }

    /// `reader` reads `key`; the stop of a watcher to start, when none ran for it.
    pub fn lease(&mut self, key: &WatchKey, reader: &str) -> Option<&Stop> {
        self.watched.lease(key, reader, Watched::default)
    }

    /// `reader` reads nothing any more; the keys it leaves without a reader.
    pub fn release(&mut self, reader: &str) -> Vec<(WatchKey, u64)> {
        self.watched.release(reader)
    }

    /// The watcher stopped and its rows dropped, if nothing reads it still.
    pub fn drop_unread(&mut self, key: &WatchKey, released: u64) {
        self.watched.drop_unread(key, released);
    }

    pub fn due(&self) -> Option<Timestamp> {
        self.due
    }

    /// Every time cell that turned by `now` written again.
    pub fn age(&mut self, now: Timestamp) {
        let mut due: Option<Timestamp> = None;
        for one in self.watched.values_mut() {
            let mut wrote = false;
            for row in &mut one.rows {
                for aging in &mut row.aging {
                    if aging.turn <= now {
                        let (text, turn) = aging.at(now);
                        if let Some(cell) = row.cells.get_mut(aging.cell) {
                            *cell = text;
                        }
                        (aging.turn, wrote) = (turn, true);
                    }
                    due = Some(due.map_or(aging.turn, |at| at.min(aging.turn)));
                }
            }
            one.revision += u64::from(wrote);
        }
        self.due = due;
    }

    /// A batch of a watcher still held; one dropped meanwhile is ignored.
    pub fn apply(&mut self, key: &WatchKey, batch: Batch) {
        let Some(one) = self.watched.get_mut(key) else {
            return;
        };
        let aging = match &batch {
            Batch::Reset { rows, .. } => rows.iter().any(|row| !row.aging.is_empty()),
            Batch::Changes(deltas) => deltas
                .iter()
                .any(|delta| matches!(delta, Delta::Put(row) if !row.aging.is_empty())),
            _ => false,
        };
        if aging {
            self.due = Some(Timestamp::default());
        }
        one.revision += 1;
        match batch {
            Batch::Reset { columns, mut rows } => {
                rows.sort_by(|a, b| place(a).cmp(&place(b)));
                (one.columns, one.rows, one.synced) = (columns, rows, true);
            }
            Batch::Changes(deltas) => changed(one, deltas),
            Batch::Watching => one.failed = None,
            Batch::Failed(why) => one.failed = Some(why),
        }
    }
}

/// A batch in one pass: rows changed in place, the gone ones swept once, the new merged in.
fn changed(one: &mut Watched, deltas: Vec<Delta>) {
    let (mut fresh, mut gone) = (Vec::<ObjectRow>::new(), Vec::new());
    let mut born: HashMap<(String, String), usize> = HashMap::new();
    for delta in deltas {
        match delta {
            Delta::Columns(columns) => one.columns = columns,
            Delta::Put(row) => match one
                .rows
                .binary_search_by(|held| place(held).cmp(&place(&row)))
            {
                Ok(at) => one.rows[at] = row,
                Err(_) => {
                    let (namespace, name) = place(&row);
                    let at = (namespace.to_string(), name.to_string());
                    match born.get(&at) {
                        Some(at) => fresh[*at] = row,
                        None => {
                            born.insert(at, fresh.len());
                            fresh.push(row);
                        }
                    }
                }
            },
            Delta::Gone(row) => gone.push(row),
        }
    }
    if !fresh.is_empty() {
        one.rows.extend(fresh);
        one.rows.sort_by(|a, b| place(a).cmp(&place(b)));
    }
    if gone.is_empty() {
        return;
    }
    let mut at: Vec<usize> = gone
        .iter()
        .filter_map(|row| {
            let found = one
                .rows
                .binary_search_by(|held| place(held).cmp(&place(row)));
            found.ok().filter(|at| one.rows[*at].uid == row.uid)
        })
        .collect();
    at.sort_unstable();
    let mut index = 0;
    one.rows.retain(|_| {
        index += 1;
        at.binary_search(&(index - 1)).is_err()
    });
}

fn place(row: &ObjectRow) -> (&str, &str) {
    (row.namespace.as_deref().unwrap_or_default(), &row.name)
}
