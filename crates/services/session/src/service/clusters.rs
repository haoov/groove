//! The clusters each session holds, written off the main thread in the order they changed.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use groove_types::{Attached, Error, SessionId, Timestamp};

use super::Service;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClusterWrite {
    Attach(Attached, Timestamp),
    Detach(Attached),
}

/// The changes waiting for a flush, and the lock one flush holds.
#[derive(Debug, Clone, Default)]
pub(super) struct Held {
    held: Arc<Mutex<VecDeque<(SessionId, ClusterWrite)>>>,
    writing: Arc<tokio::sync::Mutex<()>>,
}

impl Service {
    /// `writes` held for the next flush, after every change held before them.
    pub fn hold_clusters(&self, id: &SessionId, writes: Vec<ClusterWrite>) {
        let mut held = groove_sync::lock(&self.clusters.held);
        held.extend(writes.into_iter().map(|one| (id.clone(), one)));
    }

    /// Every change held, written in order; the first failure.
    pub async fn flush_clusters(&self) -> Result<(), Error> {
        let _writing = self.clusters.writing.lock().await;
        let held = std::mem::take(&mut *groove_sync::lock(&self.clusters.held));
        let mut flushed = Ok(());
        for (id, write) in held {
            let written = match &write {
                ClusterWrite::Attach(one, now) => self.store.attach_cluster(&id, one, *now).await,
                ClusterWrite::Detach(one) => self.store.detach_cluster(&id, one).await,
            };
            flushed = flushed.and(written.map_err(Error::from));
        }
        flushed
    }
}
