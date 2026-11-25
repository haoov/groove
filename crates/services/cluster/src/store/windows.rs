//! Each Application's sync windows from Argo CD, with when they were read: whether one is open changes.

use std::collections::{BTreeMap, BTreeSet};

use groove_types::{SyncWindows, Timestamp};

use super::follows::Named;

#[derive(Debug, Default)]
pub struct Windows {
    read: BTreeMap<Named, (Result<SyncWindows, String>, Timestamp)>,
    asking: BTreeSet<Named>,
}

impl Windows {
    pub fn get(&self, app: &Named) -> Option<&(Result<SyncWindows, String>, Timestamp)> {
        self.read.get(app)
    }

    pub fn asking(&self, app: &Named) -> bool {
        self.asking.contains(app)
    }

    /// Marks a read of `app` begun. False while one already runs.
    pub fn begin(&mut self, app: &Named) -> bool {
        self.asking.insert(app.clone())
    }

    pub fn set(&mut self, app: Named, read: Result<SyncWindows, String>, at: Timestamp) {
        self.asking.remove(&app);
        self.read.insert(app, (read, at));
    }
}
