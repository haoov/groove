//! A filesystem watcher on one worktree. Writes arrive in batches, and only the
//! directories git keeps are watched.
//!
//! A batch names what the watcher saw, not every path that moved: a file written into
//! a directory created in the same breath can beat the watch on it.

mod batch;
mod tree;

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Sender, channel};
use std::time::Duration;

use groove_types::{Error, ErrorKind, Result};

/// How long a burst must be quiet before it is reported.
pub const QUIET: Duration = Duration::from_millis(250);

/// Watching, until this is dropped.
pub struct Watch {
    _stop: Sender<()>,
    watched: Arc<AtomicUsize>,
}

impl Watch {
    /// How many directories are watched right now.
    pub fn watched(&self) -> usize {
        self.watched.load(Ordering::Relaxed)
    }
}

impl std::fmt::Debug for Watch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Watch")
    }
}

/// Calls `on_change` with what moved under `dir`, once a burst has gone quiet.
pub fn watch(
    dir: &Path,
    quiet: Duration,
    on_change: impl Fn(Vec<PathBuf>) + Send + 'static,
) -> Result<Watch> {
    let (sender, events) = channel();
    let mut watcher = notify::recommended_watcher(move |event| {
        let _ = sender.send(event);
    })
    .map_err(failed)?;
    let watched = Arc::new(AtomicUsize::new(0));
    let mut tree = tree::Tree::new(dir, watched.clone());
    if tree.reconcile(&mut watcher) == 0 {
        return Err(Error::new(
            ErrorKind::Io,
            format!("cannot watch {}", dir.display()),
        ));
    }
    let (stop, stopped) = channel();
    std::thread::spawn(move || {
        batch::report(batch::Parts {
            watcher,
            tree,
            events,
            stopped,
            quiet,
            on_change: Box::new(on_change),
        })
    });
    Ok(Watch {
        _stop: stop,
        watched,
    })
}

fn failed(e: notify::Error) -> Error {
    Error::new(ErrorKind::Io, format!("cannot watch: {e}"))
}
