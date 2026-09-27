//! The history: the commits a branch shows, and what one of them changed.

use std::collections::HashMap;

use groove_types::CommitEntry;

use crate::parse::{Counts, FORMAT, commits, numstat};
use crate::{Git, Result};

impl Git {
    /// The newest commits of the checked-out branch, the base's own marked.
    pub async fn log(&self, base: Option<&str>, limit: usize) -> Result<Vec<CommitEntry>> {
        let count = limit.to_string();
        let format = format!("--format={FORMAT}");
        let out = self
            .text(&["log", "-z", "--no-color", &format, "-n", &count, "HEAD"])
            .await?;
        let mut entries = commits(&out);
        let own = self.ahead_of(base).await?;
        for (at, entry) in entries.iter_mut().enumerate() {
            entry.is_base = at >= own;
        }
        Ok(entries)
    }

    /// How many commits the branch has that its base does not.
    async fn ahead_of(&self, base: Option<&str>) -> Result<usize> {
        let Some(base) = base else {
            return Ok(usize::MAX);
        };
        let range = format!("{base}..HEAD");
        let out = self
            .text(&["rev-list", "--count", &range])
            .await
            .unwrap_or_default();
        Ok(out.trim().parse().unwrap_or(0))
    }

    /// Lines added and deleted per path in one commit, against its first parent.
    pub async fn changed_in(&self, sha: &str) -> Result<Vec<Counts>> {
        let out = self
            .text(&["show", "--numstat", "-z", "--format=", "--no-color", sha])
            .await?;
        Ok(numstat(&out))
    }

    /// Both sides of every path a commit touched: the parent's and the commit's.
    pub async fn sides_in(
        &self,
        sha: &str,
        paths: &[String],
    ) -> Result<(HashMap<String, String>, HashMap<String, String>)> {
        let after = self.blobs(sha, paths).await?;
        let parent = format!("{sha}^");
        let before = match self.ref_exists(&parent).await? {
            true => self.blobs(&parent, paths).await?,
            false => HashMap::new(),
        };
        Ok((before, after))
    }
}
