//! A maintenance task: the flag and the children it spawns.

use groove_types::{Error, Session, SessionId, Timestamp};

use super::Service;

impl Service {
    pub async fn set_maintenance(&self, id: &SessionId, on: bool) -> Result<(), Error> {
        Ok(self.store.set_maintenance(id, on).await?)
    }

    /// Inserts the explorer, names its parent and puts it on the rail.
    pub async fn create_child(
        &self,
        session: &Session,
        parent: &SessionId,
        now: Timestamp,
    ) -> Result<(), Error> {
        self.store.create_explorer(session).await?;
        self.store.set_parent(&session.id, parent).await?;
        self.on_rail(&session.id, now).await
    }
}
