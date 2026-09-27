//! The directories a watch covers, and which of git's own files are worth a look.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use ignore::WalkBuilder;
use notify::{RecommendedWatcher, RecursiveMode, Watcher as _};

/// The watched directories, one watch each, each reporting only its own entries.
pub(crate) struct Tree {
    root: PathBuf,
    /// Watched whatever the walk says.
    fixed: Vec<PathBuf>,
    watched: HashSet<PathBuf>,
    size: Arc<AtomicUsize>,
}

impl Tree {
    pub(crate) fn new(root: &Path, fixed: Vec<PathBuf>, size: Arc<AtomicUsize>) -> Self {
        Self {
            root: root.to_path_buf(),
            fixed,
            watched: HashSet::new(),
            size,
        }
    }

    /// Watches every directory the ignore rules keep and drops the rest; returns how many.
    pub(crate) fn reconcile(&mut self, watcher: &mut RecommendedWatcher) -> usize {
        let mut wanted: HashSet<PathBuf> = directories(&self.root).into_iter().collect();
        wanted.extend(self.fixed.iter().filter(|dir| dir.is_dir()).cloned());
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

    /// Whether a path is worth reporting: not in an unwatched directory, and of git only what moves.
    pub(crate) fn keeps(&self, path: &Path) -> bool {
        if self.fixed.iter().any(|dir| path.starts_with(dir)) {
            return git_state(path);
        }
        !path.is_dir() || self.watched.contains(path)
    }
}

/// The names in git's own directory that a commit, a stage or a checkout moves.
fn git_state(path: &Path) -> bool {
    let name = path.file_name().unwrap_or_default();
    let refs = path.components().any(|part| part.as_os_str() == "refs");
    refs || name == "HEAD" || name == "index"
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
