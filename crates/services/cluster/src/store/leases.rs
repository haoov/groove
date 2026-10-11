//! What each stream holds, who reads it, and the stop that ends it once nobody does.

use std::collections::{BTreeMap, BTreeSet};

use groove_objects::Stop;

/// One stream's own state beside its readers.
#[derive(Debug)]
struct Leased<T> {
    held: T,
    readers: BTreeSet<String>,
    /// Moves on each time the last reader lets go.
    released: u64,
    stop: Stop,
}

/// The streams of one kind, by key.
#[derive(Debug)]
pub struct Leases<K, T> {
    held: BTreeMap<K, Leased<T>>,
}

impl<K, T> Default for Leases<K, T> {
    fn default() -> Self {
        Self {
            held: BTreeMap::new(),
        }
    }
}

impl<K: Ord + Clone, T> Leases<K, T> {
    pub fn get(&self, key: &K) -> Option<&T> {
        self.held.get(key).map(|one| &one.held)
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut T> {
        self.held.get_mut(key).map(|one| &mut one.held)
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.held.values_mut().map(|one| &mut one.held)
    }

    /// Every key `reader` reads.
    pub fn read_by<'a>(&'a self, reader: &'a str) -> impl Iterator<Item = &'a K> {
        let reads =
            move |(key, one): (&'a K, &'a Leased<T>)| one.readers.contains(reader).then_some(key);
        self.held.iter().filter_map(reads)
    }

    /// `reader` reads `key`, made by `made` when new; the stop of a stream to start then.
    pub fn lease(&mut self, key: &K, reader: &str, made: impl FnOnce() -> T) -> Option<&Stop> {
        let started = !self.held.contains_key(key);
        let one = self.held.entry(key.clone()).or_insert_with(|| Leased {
            held: made(),
            readers: BTreeSet::new(),
            released: 0,
            stop: Stop::new(),
        });
        one.readers.insert(reader.to_string());
        started.then_some(&one.stop)
    }

    /// `reader` reads nothing any more; the keys it leaves without a reader, each with its count.
    pub fn release(&mut self, reader: &str) -> Vec<(K, u64)> {
        let mut unread = Vec::new();
        for (key, one) in &mut self.held {
            if one.readers.remove(reader) && one.readers.is_empty() {
                one.released += 1;
                unread.push((key.clone(), one.released));
            }
        }
        unread
    }

    /// The stream stopped and taken out, if nothing read it since release `released`.
    pub fn drop_unread(&mut self, key: &K, released: u64) -> Option<T> {
        let unread = self
            .held
            .get(key)
            .is_some_and(|one| one.readers.is_empty() && one.released == released);
        if !unread {
            return None;
        }
        let one = self.held.remove(key)?;
        one.stop.stop();
        Some(one.held)
    }
}
