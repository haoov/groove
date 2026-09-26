//! The review queue: what every host of the pool asks this user to look at.

use groove_forge::Remote;
use groove_types::{Error, PoolEntry, ReviewMr};

use super::Service;

impl Service {
    /// Every host's queue, newest first, each MR with where the pool holds its repo.
    /// A host that fails is named in the errors and the others still answer.
    pub async fn review_queue(pool: &[PoolEntry]) -> (Vec<ReviewMr>, Vec<Error>) {
        let (mut found, mut failed) = (Vec::new(), Vec::new());
        for host in hosts(pool) {
            match queue_of(&host).await {
                Ok(queue) => found.extend(queue),
                Err(e) => failed.push(e),
            }
        }
        found.sort_by_key(|mr: &ReviewMr| std::cmp::Reverse(mr.updated_at.seconds()));
        let found = found.into_iter().map(|mr| local(mr, pool)).collect();
        (found, failed)
    }
}

async fn queue_of(host: &str) -> groove_types::Result<Vec<ReviewMr>> {
    Ok(Remote::of_host(host)?.review_queue().await?)
}

/// The hosts of the pool's clones, each once.
pub(crate) fn hosts(pool: &[PoolEntry]) -> Vec<String> {
    let mut out: Vec<String> = pool
        .iter()
        .filter_map(|entry| entry.slug.split('/').next())
        .map(str::to_string)
        .collect();
    out.sort();
    out.dedup();
    out
}

/// Where the MR's own repo sits in the pool, when it is there at all.
pub(crate) fn local(mut mr: ReviewMr, pool: &[PoolEntry]) -> ReviewMr {
    mr.local_path = pool
        .iter()
        .find(|entry| entry.holds(&mr.project))
        .map(|entry| entry.path.to_string_lossy().into_owned());
    mr
}
