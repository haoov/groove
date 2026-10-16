//! The directories a watch covers, and which of git's own files are worth a look.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use notify::{RecommendedWatcher, RecursiveMode, Watcher as _};

/// The watched directories, one watch each, each reporting only its own entries.
pub(crate) struct Tree {
    root: PathBuf,
    /// Watched whatever the walk says.
    fixed: Vec<PathBuf>,
    watched: HashSet<PathBuf>,
    /// The same two by their real paths, which is how macOS names what moved: `/var` is
    /// `/private/var`.
    real_fixed: Vec<PathBuf>,
    real_watched: HashSet<PathBuf>,
    size: Arc<AtomicUsize>,
}

impl Tree {
    pub(crate) fn new(root: &Path, fixed: Vec<PathBuf>, size: Arc<AtomicUsize>) -> Self {
        Self {
            root: root.to_path_buf(),
            real_fixed: fixed.iter().map(|dir| real(dir)).collect(),
            fixed,
            watched: HashSet::new(),
            real_watched: HashSet::new(),
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
        self.real_watched = self.watched.iter().map(|dir| real(dir)).collect();
        self.size.store(self.watched.len(), Ordering::Relaxed);
        self.watched.len()
    }

    /// Whether a path is worth reporting: not in an unwatched directory, and of git only what moves.
    pub(crate) fn keeps(&self, path: &Path) -> bool {
        let path = real(path);
        if self.real_fixed.iter().any(|dir| path.starts_with(dir)) {
            return git_state(&path);
        }
        !path.is_dir() || self.real_watched.contains(&path)
    }
}

/// `path` with its links resolved; as it stands when it is gone.
fn real(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

/// The names in git's own directory that a commit, a stage or a checkout moves.
fn git_state(path: &Path) -> bool {
    let name = path.file_name().unwrap_or_default();
    let refs = path.components().any(|part| part.as_os_str() == "refs");
    refs || name == "HEAD" || name == "index"
}

/// `root` and the directories under it, minus what git is told to ignore and `.git`.
fn directories(root: &Path) -> Vec<PathBuf> {
    groove_grep::walker(root)
        .build()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_some_and(|kind| kind.is_dir()))
        .map(|entry| entry.into_path())
        .collect()
}
