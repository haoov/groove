//! The commit that turns the index into history.

use crate::{Git, Result};

impl Git {
    /// Commits the index. Nothing staged is an error, as git says it is.
    pub async fn commit(&self, message: &str) -> Result<()> {
        self.text(&["commit", "-m", message]).await.map(drop)
    }
}
