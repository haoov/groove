//! An Argo CD Application's drift held where the diff reads: its files instead of the worktree's.

use std::collections::BTreeMap;

use groove_diff::{Changes, Document};

use crate::{Painted, State};

/// The Application, by its hub context, namespace and name.
pub type DriftKey = (String, String, String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Drift {
    pub app: DriftKey,
    /// The Application's resourceVersion it was read at.
    pub at: String,
    in_sync: bool,
}

/// The drift read out, and the last read that failed with what it said, each by Application and version.
#[derive(Debug, Default)]
pub struct DriftRead {
    pub asking: Option<(DriftKey, String)>,
    pub failed: Option<(DriftKey, String, String)>,
}

/// A drift made ready off the main thread: the stream, each file's counts, and both sides coloured.
#[derive(Debug)]
pub struct Prepared {
    files: Vec<groove_types::FileDiff>,
    changes: Changes,
    coloured: BTreeMap<String, Painted>,
}

/// Every file's rows and colours from its live and desired YAML; slow on a large Application.
pub fn prepare(pairs: Vec<(String, String, String)>) -> Prepared {
    let coloured = pairs
        .iter()
        .map(|(path, old, new)| {
            let sides = Painted {
                old: Document::new(path, old),
                new: Document::new(path, new),
            };
            (path.clone(), sides)
        })
        .collect();
    let (files, changes) = groove_diff::from_pairs(pairs);
    Prepared {
        files,
        changes,
        coloured,
    }
}

/// `prepare` run on a thread of its own.
pub async fn prepared(pairs: Vec<(String, String, String)>) -> groove_types::Result<Prepared> {
    groove_sync::blocking(move || Ok(prepare(pairs))).await
}

impl State {
    /// The drift shown in place of the worktree's change, its folds and scroll kept for the same Application.
    pub fn drifted(&mut self, (app, at): (DriftKey, String), prepared: Prepared) {
        let same = self.drift.as_ref().is_some_and(|one| one.app == app);
        let shut = if same {
            self.changes.folds()
        } else {
            Default::default()
        };
        self.moved();
        self.commit = None;
        self.files = prepared.files;
        self.changes = prepared.changes;
        self.changes.refold(shut);
        self.coloured = prepared.coloured;
        if !same {
            self.showing = 0..0;
        }
        let in_sync = self.files.is_empty();
        self.drift = Some(Drift { app, at, in_sync });
    }

    /// The worktree's change shown again; it reads anew.
    pub fn undrift(&mut self) -> bool {
        if self.drift.take().is_none() {
            return false;
        }
        self.moved();
        self.files.clear();
        self.changes = Changes::default();
        self.coloured.clear();
        self.showing = 0..0;
        true
    }
}

/// Nothing differs: an Application in sync.
pub fn in_sync(drift: &Drift) -> bool {
    drift.in_sync
}
