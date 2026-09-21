//! The review queue: what every host the pool knows asks this user to look at.

use groove_types::{PoolEntry, ReviewMr};
use groove_workspace_service::Service;

use crate::{AppState, Continuation, Services, Spawner};

/// Every host's queue, read at once and gathered into the board's column.
pub(super) fn read(state: &mut AppState, spawner: &dyn Spawner) {
    let hosts = hosts(state);
    if hosts.is_empty() {
        state.workspace.reviews.clear();
        return;
    }
    let pool = state.session.pool.clone();
    let job = state.begin("reading the review queue");
    spawner.spawn(Box::pin(async move {
        let mut found: Vec<ReviewMr> = Vec::new();
        let mut failed = Vec::new();
        for host in hosts {
            match Service::review_queue(&host).await {
                Ok(queue) => found.extend(queue),
                Err(e) => failed.push(e),
            }
        }
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            state.end(job);
            found.sort_by_key(|mr| std::cmp::Reverse(mr.updated_at.seconds()));
            state.workspace.reviews = found.into_iter().map(|mr| local(mr, &pool)).collect();
            state.errors.extend(failed);
        }) as Continuation
    }));
}

/// The hosts of the pool's repos, each once.
pub(crate) fn hosts(state: &AppState) -> Vec<String> {
    let mut out: Vec<String> = state
        .session
        .pool
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
