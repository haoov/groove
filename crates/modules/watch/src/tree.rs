use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use ignore::WalkBuilder;
use notify::{RecommendedWatcher, RecursiveMode, Watcher as _};

/// The directories being watched, one watch each. A watch reports the entries of its
/// own directory only.
pub(crate) struct Tree {
    root: PathBuf,
    watched: HashSet<PathBuf>,
    size: Arc<AtomicUsize>,
}

impl Tree {
    pub(crate) fn new(root: &Path, size: Arc<AtomicUsize>) -> Self {
        Self {
            root: root.to_path_buf(),
            watched: HashSet::new(),
            size,
        }
    }

    /// Watches every directory under the root the ignore rules keep, and drops the
    /// rest. Returns how many are watched.
    pub(crate) fn reconcile(&mut self, watcher: &mut RecommendedWatcher) -> usize {
        let wanted: HashSet<PathBuf> = directories(&self.root).into_iter().collect();
        let fresh: Vec<PathBuf> = wanted.difference(&self.watched).cloned().collect();
        let gone: Vec<PathBuf> = self.watched.difference(&wanted).cloned().collect();
        for dir in fresh {
            if watcher.watch(&dir, RecursiveMode::NonRecursive).is_ok() {
                self.watched.insert(dir);
            }
        }
        for dir in gone {
            let _ = watcher.unwatch(&dir);
            self.watched.remove(&dir);
        }
        self.size.store(self.watched.len(), Ordering::Relaxed);
        self.watched.len()
    }

    /// Whether a path is worth reporting: anything but a directory left unwatched.
    pub(crate) fn keeps(&self, path: &Path) -> bool {
        !path.is_dir() || self.watched.contains(path)
    }
}

/// `root` and the directories under it, minus what git is told to ignore and `.git`.
fn directories(root: &Path) -> Vec<PathBuf> {
    WalkBuilder::new(root)
        .hidden(false)
        .filter_entry(|entry| entry.file_name() != ".git")
        .build()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_some_and(|kind| kind.is_dir()))
        .map(|entry| entry.into_path())
        .collect()
}
