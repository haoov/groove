use crate::parse::{Change, porcelain};
use crate::{Error, Git, Result};

impl Git {
    /// Every changed path, index and worktree letters as git wrote them.
    pub async fn status(&self) -> Result<Vec<Change>> {
        let out = self.text(&["status", "--porcelain"]).await?;
        Ok(porcelain(&out))
    }

    /// Commits on HEAD that `base` lacks.
    pub async fn commits_since(&self, base: &str) -> Result<u32> {
        let range = format!("{base}..HEAD");
        let args = ["rev-list", "--count", &range];
        let out = self.line(&args).await?;
        out.parse().map_err(|_| Error::Unexpected {
            command: crate::command::describe(&args),
            output: out.clone(),
        })
    }

    /// Commits this branch is ahead of and behind its own head on origin; `None` when
    /// the branch was never pushed.
    pub async fn ahead_behind(&self, branch: &str) -> Result<Option<(u32, u32)>> {
        let upstream = format!("origin/{branch}");
        if !self.ref_exists(&upstream).await? {
            return Ok(None);
        }
        let range = format!("HEAD...{upstream}");
        let args = ["rev-list", "--left-right", "--count", &range];
        let out = self.line(&args).await?;
        let mut parts = out.split_whitespace().map(|n| n.parse::<u32>().ok());
        match (parts.next().flatten(), parts.next().flatten()) {
            (Some(ahead), Some(behind)) => Ok(Some((ahead, behind))),
            _ => Err(Error::Unexpected {
                command: crate::command::describe(&args),
                output: out,
            }),
        }
    }
}
