//! The transcript Claude Code writes for a session, followed from its start as it grows.

mod outcome;
mod parse;

#[cfg(test)]
mod tests;

use std::path::PathBuf;
use std::time::Duration;

use groove_types::chat::Heard;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio::sync::watch;

pub use parse::{detail, line};

/// How long the follower waits before it reads again.
const PACE: Duration = Duration::from_millis(150);

/// Ends its follower when dropped.
#[derive(Debug)]
pub struct Stop {
    _held: watch::Sender<()>,
}

/// What the follower listens to.
pub struct Heeded(watch::Receiver<()>);

pub fn stop() -> (Stop, Heeded) {
    let (sender, receiver) = watch::channel(());
    (Stop { _held: sender }, Heeded(receiver))
}

impl Heeded {
    /// Waits `wait`; false once its `Stop` is gone.
    async fn rest(&mut self, wait: Duration) -> bool {
        tokio::select! {
            _ = self.0.changed() => false,
            _ = tokio::time::sleep(wait) => true,
        }
    }
}

/// Reads `path` from its start, then what it gains, until `heeded` stops; each read's records go through `send`.
pub async fn follow(path: PathBuf, mut heeded: Heeded, send: impl Fn(Vec<Heard>) + Send) {
    let (mut read, mut carried) = (0u64, Vec::new());
    loop {
        if let Ok(gained) = gained(&path, &mut read).await {
            carried.extend(gained);
            let heard = complete(&mut carried);
            if !heard.is_empty() {
                send(heard);
            }
        }
        if !heeded.rest(PACE).await {
            return;
        }
    }
}

/// The bytes written past `read`, which moves to the end.
async fn gained(path: &PathBuf, read: &mut u64) -> std::io::Result<Vec<u8>> {
    let mut file = tokio::fs::File::open(path).await?;
    if file.metadata().await?.len() < *read {
        *read = 0;
    }
    file.seek(std::io::SeekFrom::Start(*read)).await?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).await?;
    *read += bytes.len() as u64;
    Ok(bytes)
}

/// What the whole lines say; a line still being written stays carried.
fn complete(carried: &mut Vec<u8>) -> Vec<Heard> {
    let Some(end) = carried.iter().rposition(|byte| *byte == b'\n') else {
        return Vec::new();
    };
    let whole: Vec<u8> = carried.drain(..=end).collect();
    String::from_utf8_lossy(&whole)
        .lines()
        .flat_map(parse::line)
        .collect()
}
