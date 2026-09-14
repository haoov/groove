use groove_sessions::Store;
use groove_types::{Error, Session, SessionId, SessionState, Timestamp};

/// The session capability's module handles, cheap to clone into a job.
#[derive(Clone)]
pub struct Service {
    store: Store,
}

impl Service {
    pub fn new(store: Store) -> Self {
        Self { store }
    }

    /// On a private in-memory database, for tests up the stack.
    pub async fn in_memory() -> Result<Self, Error> {
        Ok(Self::new(Store::in_memory().await?))
    }

    /// Inserts the explorer and puts it on the rail.
    pub async fn create_explorer(&self, session: &Session, now: Timestamp) -> Result<(), Error> {
        self.store.create_explorer(session).await?;
        self.store.set_opened(&session.id, Some(now)).await?;
        self.store.set_seen(&session.id, now).await?;
        Ok(())
    }

    pub async fn rename_explorer(&self, id: &SessionId, title: &str) -> Result<(), Error> {
        Ok(self.store.rename_explorer(id, title).await?)
    }

    pub async fn remove(&self, id: &SessionId) -> Result<(), Error> {
        Ok(self.store.remove(id).await?)
    }

    pub async fn set_opened(&self, id: &SessionId, at: Option<Timestamp>) -> Result<(), Error> {
        Ok(self.store.set_opened(id, at).await?)
    }

    pub async fn set_seen(&self, id: &SessionId, at: Timestamp) -> Result<(), Error> {
        Ok(self.store.set_seen(id, at).await?)
    }

    /// The rail as it was: every session with an `opened_at`, in that order.
    pub async fn opened(&self) -> Result<Vec<(Session, SessionState)>, Error> {
        Ok(self.store.opened().await?)
    }
}
