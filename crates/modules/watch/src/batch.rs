//! What the watcher holds back: a burst of paths, reported once it goes quiet.

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, RecvTimeoutError, TryRecvError};
use std::time::{Duration, Instant};

use notify::{EventKind, RecommendedWatcher};

use crate::tree::Tree;

/// A burst open this many quiet windows reports without waiting for the quiet.
const HOLD: u32 = 4;

/// Paths a batch holds before it reports, whatever the quiet.
const PATHS: usize = 256;

/// What the watching thread owns.
pub(crate) struct Parts {
    pub(crate) watcher: RecommendedWatcher,
    pub(crate) tree: Tree,
    pub(crate) events: Receiver<notify::Result<notify::Event>>,
    pub(crate) stopped: Receiver<()>,
    pub(crate) quiet: Duration,
    pub(crate) on_change: Box<dyn Fn(Vec<PathBuf>) + Send>,
}

/// Batches changes until they go quiet, whatever reads go on, then follows what appeared.
pub(crate) fn report(mut parts: Parts) {
    let mut paths: Vec<PathBuf> = Vec::new();
    let (mut opened, mut last): (Option<Instant>, Option<Instant>) = (None, None);
    loop {
        if matches!(parts.stopped.try_recv(), Err(TryRecvError::Disconnected)) {
            return;
        }
        match parts.events.recv_timeout(parts.quiet) {
            Ok(Ok(event)) if is_change(&event.kind) => {
                paths.extend(event.paths);
                let now = Instant::now();
                opened.get_or_insert(now);
                last = Some(now);
            }
            Ok(_) | Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        let quiet = last.is_some_and(|at| at.elapsed() >= parts.quiet);
        let long = opened.is_some_and(|at| at.elapsed() >= parts.quiet * HOLD);
        if paths.is_empty() || !(quiet || long || paths.len() >= PATHS) {
            continue;
        }
        (opened, last) = (None, None);
        let batch = batch(&mut paths);
        if batch.iter().any(|path| path.is_dir() || !path.exists()) {
            parts.tree.reconcile(&mut parts.watcher);
        }
        let batch: Vec<PathBuf> = batch
            .into_iter()
            .filter(|path| parts.tree.keeps(path))
            .collect();
        if !batch.is_empty() {
            (parts.on_change)(batch);
        }
    }
}

fn batch(paths: &mut Vec<PathBuf>) -> Vec<PathBuf> {
    let mut batch = std::mem::take(paths);
    batch.sort();
    batch.dedup();
    batch
}

/// Creates, writes, renames and removals; opening a file is not a change.
fn is_change(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) | EventKind::Any
    )
}
