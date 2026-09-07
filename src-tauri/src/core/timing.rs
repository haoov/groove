//! Wall-clock timing for subprocesses and network calls. Enable with `RUST_LOG=timing=debug`.

use std::time::Instant;

/// Minimum duration that produces a log line.
const NOISE_FLOOR_MS: u128 = 2;

pub async fn timed<T, F>(op: &'static str, detail: impl AsRef<str>, fut: F) -> T
where
    F: std::future::Future<Output = T>,
{
    let started = Instant::now();
    let out = fut.await;
    let elapsed = started.elapsed();
    if elapsed.as_millis() >= NOISE_FLOOR_MS {
        tracing::debug!(target: "timing", "{op} [{}] {elapsed:?}", detail.as_ref());
    }
    out
}
