//! A database reached with a token, before it becomes a source.

use groove_types::NotionConfig;

use super::Notion;
use crate::Result;

impl Notion {
    /// The source a token, a database and a user make, once the token reads the database.
    pub async fn connect(token: &str, database_id: &str, user_id: &str) -> Result<NotionConfig> {
        Self::connect_at(super::HOST, token, database_id, user_id).await
    }

    /// The same, against another host, which the tests answer on.
    pub async fn connect_at(
        host: &str,
        token: &str,
        database_id: &str,
        user_id: &str,
    ) -> Result<NotionConfig> {
        let config = NotionConfig::bare(token, database_id, user_id);
        let notion = Notion::at(host, config.clone())?;
        notion.database(&config.database_id).await?;
        Ok(config)
    }
}
