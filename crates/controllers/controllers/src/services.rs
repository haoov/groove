/// The services' module handles, built by the binary and read by every controller.
#[derive(Clone)]
pub struct Services {
    pub session: groove_session_service::Service,
}

impl Services {
    /// Every service on a private in-memory database, for tests.
    pub async fn in_memory() -> groove_types::Result<Self> {
        Ok(Self {
            session: groove_session_service::Service::in_memory().await?,
        })
    }
}
