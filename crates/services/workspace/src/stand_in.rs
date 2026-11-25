//! A change shown in place of the worktree's: an Argo CD Application's drift, or a cluster write under review.

use std::collections::BTreeMap;

use groove_diff::{Changes, Document};
use groove_types::ApprovalId;

use crate::{DriftKey, Painted, State};

/// What the change standing in comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// An Application's drift, read at its resourceVersion.
    Drift { app: DriftKey, at: String },
    /// The cluster write waiting on the user.
    Write(ApprovalId),
}

impl Source {
    /// The same Application, or the same write, whatever its version.
    fn same(&self, other: &Source) -> bool {
        match (self, other) {
            (Source::Drift { app, .. }, Source::Drift { app: other, .. }) => app == other,
            (Source::Write(one), Source::Write(other)) => one == other,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StandIn {
    pub source: Source,
    empty: bool,
}

impl StandIn {
    /// Nothing differs: an Application in sync.
    pub fn empty(&self) -> bool {
        self.empty
    }
}

/// A change made ready off the main thread: the stream, each file's counts, and both sides coloured.
#[derive(Debug)]
pub struct Prepared {
    files: Vec<groove_types::FileDiff>,
    changes: Changes,
    coloured: BTreeMap<String, Painted>,
}

/// Every file's rows and colours from its two sides; slow on a large Application.
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
    /// `prepared` shown in place of the worktree's change, its folds and scroll kept for the same source.
    pub fn show_instead(&mut self, source: Source, prepared: Prepared) {
        let held = self.stand_in.as_ref();
        let same = held.is_some_and(|one| one.source.same(&source));
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
        let empty = self.files.is_empty();
        self.stand_in = Some(StandIn { source, empty });
    }

    /// The worktree's change shown again; false when nothing stood in for it.
    pub fn show_worktree(&mut self) -> bool {
        if self.stand_in.take().is_none() {
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
