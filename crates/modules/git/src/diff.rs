use crate::parse::{Counts, numstat};
use crate::{Git, Result};

impl Git {
    /// Lines added and deleted per path, between `rev` and the working tree.
    pub async fn numstat(&self, rev: &str) -> Result<Vec<Counts>> {
        let out = self.text(&["diff", "--numstat", "-z", rev]).await?;
        Ok(numstat(&out))
    }

    /// The file's content at `rev`, for the side of a diff that is not on disk.
    pub async fn show(&self, rev: &str, path: &str) -> Result<String> {
        self.text(&["show", &format!("{rev}:{path}")]).await
    }
}
