//! A cache in two halves: when the new one fills, the old one goes, and what was asked for since stays.

use std::collections::HashMap;

pub(crate) struct Kept<V> {
    new: HashMap<Box<str>, V>,
    old: HashMap<Box<str>, V>,
    cap: usize,
}

impl<V: Copy> Kept<V> {
    pub(crate) fn new(cap: usize) -> Self {
        Self {
            new: HashMap::new(),
            old: HashMap::new(),
            cap,
        }
    }

    /// The value kept for `key`, moved to the new half when the old one held it.
    pub(crate) fn get(&mut self, key: &str) -> Option<V> {
        if let Some(value) = self.new.get(key) {
            return Some(*value);
        }
        let (key, value) = self.old.remove_entry(key)?;
        self.insert(key, value);
        Some(value)
    }

    /// The value, without moving it.
    pub(crate) fn peek(&self, key: &str) -> Option<V> {
        self.new.get(key).or_else(|| self.old.get(key)).copied()
    }

    pub(crate) fn insert(&mut self, key: Box<str>, value: V) {
        if self.new.len() >= self.cap {
            self.old = std::mem::take(&mut self.new);
        }
        self.new.insert(key, value);
    }
}
