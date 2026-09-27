//! A watcher on one worktree, in batches, over the directories git keeps.
//! A batch names what was seen: a file written into a new directory can beat its watch.

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

/// How long a burst stays quiet before it is reported: longer than a write and its rename.
pub const QUIET: Duration = Duration::from_millis(25);

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

/// Calls `on_change` with what moved under `dir` or in `also`, once a burst is quiet.
pub fn watch(
    dir: &Path,
    also: Vec<PathBuf>,
    quiet: Duration,
    on_change: impl Fn(Vec<PathBuf>) + Send + 'static,
) -> Result<Watch> {
    let (sender, events) = channel();
    let mut watcher = notify::recommended_watcher(move |event| {
        let _ = sender.send(event);
    })
    .map_err(failed)?;
    let watched = Arc::new(AtomicUsize::new(0));
    let mut tree = tree::Tree::new(dir, also, watched.clone());
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
