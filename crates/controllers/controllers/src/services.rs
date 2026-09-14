/// The services' module handles, built by the binary and read by every controller.
#[derive(Clone)]
pub struct Services {
    pub session: groove_session_service::Service,
}

impl Services {
    /// Every service on a private in-memory database, the pool under `root`, for tests.
    pub async fn in_memory(root: &std::path::Path) -> groove_types::Result<Self> {
        Ok(Self {
            session: groove_session_service::Service::in_memory(root).await?,
        })
    }
}
