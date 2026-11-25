//! The revisions an Application's sources name, each read once from Argo CD: a commit does not change.

use std::collections::BTreeMap;

use groove_types::RevisionMeta;

use super::follows::Named;

/// An Application, its source when it has several, and a revision of it.
pub type Revised = (Named, Option<usize>, String);

#[derive(Debug, Default)]
pub struct Revisions {
    /// None while its read runs.
    read: BTreeMap<Revised, Option<Result<RevisionMeta, String>>>,
}

impl Revisions {
    /// What was read of `one`, or why not; none while asked or never asked.
    pub fn get(&self, one: &Revised) -> Option<&Result<RevisionMeta, String>> {
        self.read.get(one)?.as_ref()
    }

    /// Marks `one` asked. False when it was asked before.
    pub fn begin(&mut self, one: &Revised) -> bool {
        if self.read.contains_key(one) {
            return false;
        }
        self.read.insert(one.clone(), None);
        true
    }

    pub fn asked(&self, one: &Revised) -> bool {
        self.read.contains_key(one)
    }

    pub fn set(&mut self, one: Revised, read: Result<RevisionMeta, String>) {
        self.read.insert(one, Some(read));
    }
}
