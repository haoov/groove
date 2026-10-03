//! Across a worktree: the lines holding a text, and the paths themselves.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use ignore::{WalkBuilder, WalkState};

#[cfg(test)]
mod tests;

/// How many matches gather before they are reported.
const BATCH: usize = 64;

/// One line a search matched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    /// The file, from the worktree root.
    pub path: String,
    pub line: usize,
    pub text: String,
    /// Where the query sits in the line, in characters.
    pub at: (usize, usize),
}

/// What a search running in the background answers to.
#[derive(Debug, Default)]
pub struct Search {
    stopped: AtomicBool,
    found: AtomicUsize,
}

impl Search {
    /// Nothing wants this search any more.
    pub fn stop(&self) {
        self.stopped.store(true, Ordering::Relaxed);
    }

    pub fn is_stopped(&self) -> bool {
        self.stopped.load(Ordering::Relaxed)
    }

    fn took(&self, more: usize) -> usize {
        self.found.fetch_add(more, Ordering::Relaxed) + more
    }
}

/// A walk of the worktree at `dir` that keeps hidden files, skips `.git` and keeps to the ignore rules.
pub fn walker(dir: &Path) -> WalkBuilder {
    let mut walk = WalkBuilder::new(dir);
    walk.hidden(false)
        .require_git(false)
        .filter_entry(|entry| entry.file_name() != ".git");
    walk
}

/// Every file under `dir` the ignore rules leave, at most `cap`, from the root.
pub fn paths(dir: &Path, cap: usize) -> Vec<String> {
    let mut out = Vec::new();
    let walk = walker(dir).build();
    for entry in walk.flatten() {
        if out.len() >= cap {
            break;
        }
        if !entry.metadata().is_ok_and(|meta| meta.is_file()) {
            continue;
        }
        if let Ok(path) = entry.path().strip_prefix(dir) {
            out.push(path.to_string_lossy().into_owned());
        }
    }
    out.sort();
    out
}

/// Every line under `dir` holding `query`, in batches; blocks until the walk ends or stops.
pub fn walk(
    dir: &Path,
    query: &str,
    under: &str,
    cap: usize,
    search: Arc<Search>,
    mut found: impl FnMut(Vec<Found>) + Send,
) {
    if query.is_empty() {
        return;
    }
    let (send, take) = std::sync::mpsc::channel::<Vec<Found>>();
    std::thread::scope(|scope| {
        scope.spawn(|| {
            for batch in take {
                found(batch);
            }
        });
        read(dir, query, under, cap, &search, send);
    });
}

/// The walk itself, one thread a core, each sending what its files held.
fn read(
    dir: &Path,
    query: &str,
    under: &str,
    cap: usize,
    search: &Arc<Search>,
    send: std::sync::mpsc::Sender<Vec<Found>>,
) {
    let needle = query.to_lowercase();
    walker(dir).build_parallel().run(|| {
        let (dir, needle) = (dir.to_path_buf(), needle.clone());
        let under = under.to_string();
        let (search, send) = (search.clone(), send.clone());
        Box::new(move |entry| {
            if search.is_stopped() {
                return WalkState::Quit;
            }
            let Ok(entry) = entry else {
                return WalkState::Continue;
            };
            let Some(found) = matches(&dir, &entry, &needle, &under) else {
                return WalkState::Continue;
            };
            if found.is_empty() {
                return WalkState::Continue;
            }
            let count = search.took(found.len());
            let over = count.saturating_sub(cap);
            let mut found = found;
            found.truncate(found.len().saturating_sub(over));
            if !found.is_empty() {
                let _ = send.send(found);
            }
            match count >= cap {
                true => {
                    search.stop();
                    WalkState::Quit
                }
                false => WalkState::Continue,
            }
        })
    });
}

/// What one file holds, or nothing when it is not a file worth reading.
fn matches(dir: &Path, entry: &ignore::DirEntry, needle: &str, under: &str) -> Option<Vec<Found>> {
    let meta = entry.metadata().ok()?;
    if !meta.is_file() || meta.len() > groove_types::TEXT_MAX_BYTES {
        return None;
    }
    let path = entry.path().strip_prefix(dir).ok()?.to_string_lossy();
    if !groove_types::narrows(&path, under) {
        return None;
    }
    let text = std::fs::read_to_string(entry.path()).ok()?;
    if text.contains('\0') {
        return None;
    }
    let mut found = Vec::new();
    for (line, text) in text.lines().enumerate() {
        found.extend(within(text, needle).map(|at| Found {
            path: path.to_string(),
            line,
            text: text.to_string(),
            at,
        }));
        if found.len() >= BATCH {
            break;
        }
    }
    Some(found)
}

/// Where the needle sits in the line, ignoring case, in characters.
fn within(text: &str, needle: &str) -> Option<(usize, usize)> {
    let first = groove_types::occurrences(text, needle).into_iter().next()?;
    Some((first.start, first.end))
}
