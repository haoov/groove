//! Ref answers change on commits, fetches and checkouts — not on every diff
//! refresh. A short TTL collapses the 3–5 resolutions one refresh performs
//! into one, and `flush()` restores instant freshness after every operation
//! the app itself performs. The TTL is the safety net for ref movement the
//! app cannot see (an agent committing in its own terminal).

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

const TTL: Duration = Duration::from_secs(5);

pub struct RefCache {
    ttl: Duration,
    texts: Mutex<HashMap<String, (Instant, Option<String>)>>,
    flags: Mutex<HashMap<String, (Instant, bool)>>,
    /// Per-key throttle stamps for fire-and-forget fetches (no TTL — each
    /// caller states its own window).
    stamps: Mutex<HashMap<String, Instant>>,
}

static REFS: LazyLock<RefCache> = LazyLock::new(|| RefCache::new(TTL));

pub fn shared() -> &'static RefCache {
    &REFS
}

/// Drop every cached answer. Called after anything that can move a ref:
/// commit, push, pull, rebase, discard, fetch, worktree add/close/switch.
pub fn flush() {
    REFS.clear();
}

impl RefCache {
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            texts: Mutex::new(HashMap::new()),
            flags: Mutex::new(HashMap::new()),
            stamps: Mutex::new(HashMap::new()),
        }
    }

    pub async fn text<F, Fut>(&self, key: String, compute: F) -> Option<String>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Option<String>>,
    {
        if let Ok(map) = self.texts.lock() {
            if let Some((at, value)) = map.get(&key) {
                if at.elapsed() < self.ttl {
                    return value.clone();
                }
            }
        }
        let value = compute().await;
        if let Ok(mut map) = self.texts.lock() {
            map.insert(key, (Instant::now(), value.clone()));
        }
        value
    }

    pub async fn flag<F, Fut>(&self, key: String, compute: F) -> bool
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = bool>,
    {
        if let Ok(map) = self.flags.lock() {
            if let Some((at, value)) = map.get(&key) {
                if at.elapsed() < self.ttl {
                    return *value;
                }
            }
        }
        let value = compute().await;
        if let Ok(mut map) = self.flags.lock() {
            map.insert(key, (Instant::now(), value));
        }
        value
    }

    /// True (and stamps now) when `window` has passed since the last stamp for
    /// `key` — the throttle for fire-and-forget fetches.
    pub fn due(&self, key: &str, window: Duration) -> bool {
        let Ok(mut map) = self.stamps.lock() else { return true };
        let now = Instant::now();
        match map.get(key) {
            Some(at) if now.duration_since(*at) < window => false,
            _ => {
                map.insert(key.to_string(), now);
                true
            }
        }
    }

    pub fn clear(&self) {
        if let Ok(mut map) = self.texts.lock() {
            map.clear();
        }
        if let Ok(mut map) = self.flags.lock() {
            map.clear();
        }
        // Fetch stamps survive a flush: flushing answers must not un-throttle
        // the network.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[tokio::test]
    async fn a_hit_within_ttl_never_recomputes() {
        let cache = RefCache::new(Duration::from_secs(60));
        let calls = AtomicU32::new(0);
        for _ in 0..3 {
            let got = cache
                .text("k".into(), || async {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Some("v".to_string())
                })
                .await;
            assert_eq!(got.as_deref(), Some("v"));
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn negative_answers_are_cached_too() {
        let cache = RefCache::new(Duration::from_secs(60));
        let calls = AtomicU32::new(0);
        for _ in 0..2 {
            let got = cache
                .text("missing".into(), || async {
                    calls.fetch_add(1, Ordering::SeqCst);
                    None
                })
                .await;
            assert_eq!(got, None);
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn expiry_and_clear_both_recompute() {
        let cache = RefCache::new(Duration::from_millis(1));
        let calls = AtomicU32::new(0);
        let compute = || async {
            calls.fetch_add(1, Ordering::SeqCst);
            true
        };
        assert!(cache.flag("k".into(), compute).await);
        tokio::time::sleep(Duration::from_millis(5)).await;
        assert!(cache.flag("k".into(), compute).await);
        assert_eq!(calls.load(Ordering::SeqCst), 2, "the TTL expired");

        let long = RefCache::new(Duration::from_secs(60));
        let n = AtomicU32::new(0);
        let compute = || async {
            n.fetch_add(1, Ordering::SeqCst);
            false
        };
        long.flag("k".into(), compute).await;
        long.clear();
        long.flag("k".into(), compute).await;
        assert_eq!(n.load(Ordering::SeqCst), 2, "clear() drops the entry");
    }

    #[test]
    fn the_fetch_throttle_stamps_once_per_window() {
        let cache = RefCache::new(Duration::from_secs(60));
        assert!(cache.due("repo", Duration::from_secs(60)));
        assert!(!cache.due("repo", Duration::from_secs(60)));
        assert!(cache.due("other", Duration::from_secs(60)));
    }
}
