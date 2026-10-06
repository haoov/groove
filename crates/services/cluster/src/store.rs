//! The rows of every watcher something reads, who reads each, and the kinds of each context.

use std::collections::{BTreeMap, BTreeSet};

use groove_objects::{Batch, Delta, Stop};
use groove_types::{KubeKind, ObjectRow, TableColumn, WatchKey};

/// One watcher's rows, by namespace then name, and how it stands.
#[derive(Debug, Default)]
pub struct Watched {
    pub columns: Vec<TableColumn>,
    pub rows: Vec<ObjectRow>,
    /// The first list has landed.
    pub synced: bool,
    /// Why the watcher stands still, until its next batch lands.
    pub failed: Option<String>,
    readers: BTreeSet<String>,
    stop: Stop,
}

/// The watchers and the kinds, beside the rest of the `cluster` slice.
#[derive(Debug, Default)]
pub struct Store {
    watched: BTreeMap<WatchKey, Watched>,
    kinds: BTreeMap<String, Vec<KubeKind>>,
}

impl Store {
    pub fn watched(&self, key: &WatchKey) -> Option<&Watched> {
        self.watched.get(key)
    }

    pub fn kinds(&self, context: &str) -> Option<&[KubeKind]> {
        self.kinds.get(context).map(Vec::as_slice)
    }

    pub fn set_kinds(&mut self, context: &str, kinds: Vec<KubeKind>) {
        self.kinds.insert(context.to_string(), kinds);
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

    /// A batch of a watcher still held; one dropped meanwhile is ignored.
    pub fn apply(&mut self, key: &WatchKey, batch: Batch) {
        let Some(one) = self.watched.get_mut(key) else {
            return;
        };
        match batch {
            Batch::Reset { columns, mut rows } => {
                rows.sort_by(|a, b| place(a).cmp(&place(b)));
                (one.columns, one.rows, one.synced, one.failed) = (columns, rows, true, None);
            }
            Batch::Changes(deltas) => {
                one.failed = None;
                deltas.into_iter().for_each(|delta| changed(one, delta));
            }
            Batch::Failed(why) => one.failed = Some(why),
        }
    }
}

fn changed(one: &mut Watched, delta: Delta) {
    match delta {
        Delta::Columns(columns) => one.columns = columns,
        Delta::Put(row) => match one.rows.iter().position(|held| held.uid == row.uid) {
            Some(at) => one.rows[at] = row,
            None => {
                let at = one.rows.partition_point(|held| place(held) < place(&row));
                one.rows.insert(at, row);
            }
        },
        Delta::Gone(uid) => one.rows.retain(|held| held.uid != uid),
    }
}

fn place(row: &ObjectRow) -> (&str, &str) {
    (row.namespace.as_deref().unwrap_or_default(), &row.name)
}
