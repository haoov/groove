//! Moved paths, sent to the server in batches as `workspace/didChangeWatchedFiles`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use groove_types::{Error, ErrorKind, Result};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use serde_json::{Value, json};
use tokio::sync::mpsc;

use crate::link::Link;
use crate::uri;

/// How long a burst of moves stays quiet before it is sent.
const QUIET: Duration = Duration::from_millis(100);

pub(crate) struct Watched {
    queue: mpsc::UnboundedSender<PathBuf>,
    watcher: Option<RecommendedWatcher>,
}

impl Watched {
    pub(crate) fn new(link: Arc<Link>) -> Self {
        let (queue, moved) = mpsc::unbounded_channel();
        tokio::spawn(send(link, moved));
        Self {
            queue,
            watcher: None,
        }
    }

    /// Watches each of `dirs`, its own entries only, in place of what was watched before.
    pub(crate) fn watch(&mut self, dirs: &[PathBuf]) -> Result<()> {
        let queue = self.queue.clone();
        let mut watcher =
            notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
                for path in event.map(|one| one.paths).unwrap_or_default() {
                    let _ = queue.send(path);
                }
            })
            .map_err(|e| Error::new(ErrorKind::Io, e.to_string()))?;
        for dir in dirs {
            watcher
                .watch(dir, RecursiveMode::NonRecursive)
                .map_err(|e| {
                    Error::new(
                        ErrorKind::Io,
                        format!("cannot watch {}: {e}", dir.display()),
                    )
                })?;
        }
        self.watcher = Some(watcher);
        Ok(())
    }

    pub(crate) fn moved(&self, paths: Vec<PathBuf>) {
        for path in paths {
            let _ = self.queue.send(path);
        }
    }
}

/// Each quiet burst as one notice: a directory created, a file changed, a gone path deleted.
async fn send(link: Arc<Link>, mut moved: mpsc::UnboundedReceiver<PathBuf>) {
    while let Some(first) = moved.recv().await {
        let mut burst = BTreeSet::from([first]);
        while let Ok(Some(more)) = tokio::time::timeout(QUIET, moved.recv()).await {
            burst.insert(more);
        }
        let mut changes = Vec::new();
        for path in burst {
            let kind = match tokio::fs::metadata(&path).await {
                Ok(meta) if meta.is_dir() => 1,
                Ok(_) => 2,
                Err(_) => 3,
            };
            changes.push(json!({ "uri": uri::of(&path), "type": kind }));
        }
        let params = json!({ "changes": Value::Array(changes) });
        if link
            .notify("workspace/didChangeWatchedFiles", params)
            .is_err()
        {
            return;
        }
    }
}

/// The directories `patterns` name under `root`; a `*` segment stands for any directory.
pub(crate) async fn resolved(root: &Path, patterns: &[String]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for pattern in patterns {
        let mut found = vec![root.to_path_buf()];
        for segment in pattern.split('/').filter(|one| !one.is_empty()) {
            let mut next = Vec::new();
            for dir in found {
                match segment {
                    "*" => next.extend(children(&dir).await),
                    named => next.push(dir.join(named)),
                }
            }
            found = next;
        }
        for dir in found {
            if tokio::fs::metadata(&dir)
                .await
                .is_ok_and(|meta| meta.is_dir())
            {
                out.push(dir);
            }
        }
    }
    out
}

async fn children(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(mut entries) = tokio::fs::read_dir(dir).await else {
        return out;
    };
    while let Ok(Some(entry)) = entries.next_entry().await {
        if entry.file_type().await.is_ok_and(|kind| kind.is_dir()) {
            out.push(entry.path());
        }
    }
    out
}
