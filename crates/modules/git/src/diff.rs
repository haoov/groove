use std::collections::HashMap;

use crate::parse::{Counts, batch, numstat};
use crate::{Git, Result};

impl Git {
    /// Lines added and deleted per path, between `rev` and the working tree.
    pub async fn numstat(&self, rev: &str) -> Result<Vec<Counts>> {
        let out = self.text(&["diff", "--numstat", "-z", rev]).await?;
        Ok(numstat(&out))
    }

    /// The content of every path at `rev`, read in one process.
    pub async fn blobs(&self, rev: &str, paths: &[String]) -> Result<HashMap<String, String>> {
        let wanted: Vec<&str> = paths
            .iter()
            .map(String::as_str)
            .filter(|path| !path.contains('\n'))
            .collect();
        if wanted.is_empty() {
            return Ok(HashMap::new());
        }
        let asked: String = wanted
            .iter()
            .map(|path| format!("{rev}:{path}\n"))
            .collect();
        let out = self.fed(&["cat-file", "--batch"], asked).await?;
        Ok(batch(&out.stdout, &wanted))
    }

    /// The file's content at `rev`, for the side of a diff that is not on disk.
    pub async fn show(&self, rev: &str, path: &str) -> Result<String> {
        self.text(&["show", &format!("{rev}:{path}")]).await
    }
}
