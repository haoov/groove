//! The service handles the binary builds and every controller reads.

use std::sync::Arc;

use groove_workspace_service::{Clipboard, Memory};

/// The services' module handles, built by the binary and read by every controller.
#[derive(Clone)]
pub struct Services {
    pub session: groove_session_service::Service,
    pub task: groove_task_service::Service,
    pub delivery: groove_delivery_service::Service,
    /// What every session did to its own work.
    pub timeline: groove_session_service::Timeline,
    /// What the open file copies through.
    pub clipboard: Arc<dyn Clipboard>,
}

impl Services {
    /// Every service on a private in-memory database, the pool under `root`, and a
    /// clipboard of its own, for tests.
    pub async fn in_memory(root: &std::path::Path) -> groove_types::Result<Self> {
        let session = groove_session_service::Service::in_memory(root).await?;
        Ok(Self {
            delivery: groove_delivery_service::Service::beside(session.store()),
            timeline: groove_session_service::Timeline::new(session.store().db().clone()),
            session,
            task: groove_task_service::Service::in_memory().await?,
            clipboard: Arc::new(Memory::default()),
        })
    }
}
