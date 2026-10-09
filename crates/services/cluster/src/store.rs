//! The rows of every watcher something reads, who reads each, and the kinds of each context.

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
    readers: BTreeSet<String>,
    stop: Stop,
}

/// The watchers and the kinds, beside the rest of the `cluster` slice.
#[derive(Debug, Default)]
pub struct Store {
    watched: BTreeMap<WatchKey, Watched>,
    kinds: BTreeMap<String, Vec<KubeKind>>,
    discovering: BTreeSet<String>,
    namespaces: BTreeMap<String, Vec<String>>,
    /// When a time cell next reads differently.
    due: Option<Timestamp>,
}

impl Store {
    pub fn watched(&self, key: &WatchKey) -> Option<&Watched> {
        self.watched.get(key)
    }

    pub fn kinds(&self, context: &str) -> Option<&[KubeKind]> {
        self.kinds.get(context).map(Vec::as_slice)
    }

    pub fn set_kinds(&mut self, context: &str, kinds: Vec<KubeKind>) {
        self.discovering.remove(context);
        self.kinds.insert(context.to_string(), kinds);
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

    pub fn end_discovery(&mut self, context: &str) {
        self.discovering.remove(context);
    }

    pub fn discovering(&self, context: &str) -> bool {
        self.discovering.contains(context)
    }

    /// Every key `reader` reads.
    pub fn read_by<'a>(&'a self, reader: &'a str) -> impl Iterator<Item = &'a WatchKey> {
        self.watched
            .iter()
            .filter(move |(_, one)| one.readers.contains(reader))
            .map(|(key, _)| key)
    }

    /// `reader` reads `key`; the stop of a watcher to start, when none ran for it.
    pub fn lease(&mut self, key: &WatchKey, reader: &str) -> Option<&Stop> {
        let started = !self.watched.contains_key(key);
        let one = self.watched.entry(key.clone()).or_default();
        one.readers.insert(reader.to_string());
        started.then_some(&one.stop)
    }

    /// `reader` reads nothing any more; the keys it leaves without a reader.
    pub fn release(&mut self, reader: &str) -> Vec<WatchKey> {
        let mut unread = Vec::new();
        for (key, one) in &mut self.watched {
            if one.readers.remove(reader) && one.readers.is_empty() {
                unread.push(key.clone());
            }
        }
        unread
    }

    /// The watcher stopped and its rows dropped, if nothing reads it still.
    pub fn drop_unread(&mut self, key: &WatchKey) {
        if self
            .watched
            .get(key)
            .is_some_and(|one| one.readers.is_empty())
            && let Some(one) = self.watched.remove(key)
        {
            one.stop.stop();
        }
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
    let mut born: HashMap<String, usize> = HashMap::new();
    for delta in deltas {
        match delta {
            Delta::Columns(columns) => one.columns = columns,
            Delta::Put(row) => match one
                .rows
                .binary_search_by(|held| place(held).cmp(&place(&row)))
            {
                Ok(at) => one.rows[at] = row,
                Err(_) => match born.get(&row.uid) {
                    Some(at) => fresh[*at] = row,
                    None => {
                        born.insert(row.uid.clone(), fresh.len());
                        fresh.push(row);
                    }
                },
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
