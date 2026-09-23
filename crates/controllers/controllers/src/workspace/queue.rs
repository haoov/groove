//! The review queue: what every host the pool knows asks this user to look at.

use groove_types::{PoolEntry, ReviewMr};
use groove_workspace_service::Service;

use crate::{AppState, Continuation, Services, Spawner};

/// Every host's queue, read at once and gathered into the board's column.
pub(super) fn read(state: &mut AppState, services: &Services, spawner: &dyn Spawner) {
    let service = services.session.clone();
    let job = state.begin("reading the review queue");
    spawner.spawn(Box::pin(async move {
        let pool = service.list_pool();
        let mut found: Vec<ReviewMr> = Vec::new();
        let mut failed = Vec::new();
        for host in hosts(&pool) {
            match Service::review_queue(&host).await {
                Ok(queue) => found.extend(queue),
                Err(e) => failed.push(e),
            }
        }
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            state.end(job);
            found.sort_by_key(|mr| std::cmp::Reverse(mr.updated_at.seconds()));
            state.workspace.reviews = found.into_iter().map(|mr| local(mr, &pool)).collect();
            for one in failed {
                state.failed(one);
            }
        }) as Continuation
    }));
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
