//! One client per context and set of files, made again when a file changes or a sign-in is refused.

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::SystemTime;

use crate::{Client, Result};

/// A file's modification time and length; none while it is missing.
type Stamp = Option<(SystemTime, u64)>;

struct Pooled {
    paths: Vec<PathBuf>,
    stamps: Vec<Stamp>,
    client: Client,
}

static POOL: Mutex<Vec<Pooled>> = Mutex::new(Vec::new());

impl Client {
    /// The client held for `context` while the files stand as they did, else a new one kept.
    pub async fn shared(paths: &[PathBuf], context: &str) -> Result<Self> {
        let stamps = stamps(paths.to_vec()).await;
        if let Some(held) = held(paths, context, &stamps) {
            return Ok(held);
        }
        let client = Self::connect(paths, context).await?;
        let mut pool = POOL.lock().unwrap_or_else(|e| e.into_inner());
        pool.retain(|one| one.client.context != context || one.paths != paths);
        pool.push(Pooled {
            paths: paths.to_vec(),
            stamps,
            client: client.clone(),
        });
        Ok(client)
    }
}

fn held(paths: &[PathBuf], context: &str, stamps: &[Stamp]) -> Option<Client> {
    let pool = POOL.lock().unwrap_or_else(|e| e.into_inner());
    let same = |one: &&Pooled| one.client.context == context && one.paths == paths;
    let found = pool.iter().find(same)?;
    (found.stamps == stamps).then(|| found.client.clone())
}

/// Every client of `context` dropped: the next one reads the files and signs in again.
pub(crate) fn forget(context: &str) {
    let mut pool = POOL.lock().unwrap_or_else(|e| e.into_inner());
    pool.retain(|one| one.client.context != context);
}

async fn stamps(paths: Vec<PathBuf>) -> Vec<Stamp> {
    let read = tokio::task::spawn_blocking(move || {
        let stamp = |path: &PathBuf| {
            let meta = std::fs::metadata(path).ok()?;
            Some((meta.modified().ok()?, meta.len()))
        };
        paths.iter().map(stamp).collect()
    });
    read.await.unwrap_or_default()
}
