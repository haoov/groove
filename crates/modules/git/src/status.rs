use crate::parse::{Change, porcelain};
use crate::{Error, Git, Result};

impl Git {
    /// Every changed path with its index and worktree letters, untracked files one by one.
    pub async fn status(&self) -> Result<Vec<Change>> {
        let out = self.text(&["status", "--porcelain", "-uall"]).await?;
        Ok(porcelain(&out))
    }

    /// Commits on HEAD that `base` lacks.
    pub async fn commits_since(&self, base: &str) -> Result<u32> {
        self.count(&format!("{base}..HEAD")).await
    }

    /// Commits on `branch` that origin lacks: past its copy there, else past its base.
    pub async fn unpushed(&self, branch: &str, pinned: Option<&str>) -> Result<u32> {
        let point = self.pushed_point(branch, pinned).await?;
        self.count(&format!("{point}..refs/heads/{branch}")).await
    }

    async fn count(&self, range: &str) -> Result<u32> {
        let args = ["rev-list", "--count", range];
        let out = self.line(&args).await?;
        out.parse().map_err(|_| Error::Unexpected {
            command: crate::command::describe(&args),
            output: out.clone(),
        })
    }

    /// Commits ahead of and behind the branch on origin; `None` when it was never pushed.
    pub async fn ahead_behind(&self, branch: &str) -> Result<Option<(u32, u32)>> {
        let Some(upstream) = self.remote_of(branch).await? else {
            return Ok(None);
        };
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
