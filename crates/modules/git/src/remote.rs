use std::path::Path;

use crate::{Error, Git, Result};

impl Git {
    /// `git fetch origin <refspecs…>`; no refspec fetches everything.
    pub async fn fetch(&self, refspecs: &[&str]) -> Result<()> {
        let mut args = vec!["fetch", "origin"];
        args.extend_from_slice(refspecs);
        self.text(&args).await.map(drop)
    }

    /// Every branch head on origin, asked of the remote; an error means origin is unreachable.
    pub async fn remote_heads(&self) -> Result<Vec<String>> {
        let out = self.text(&["ls-remote", "--heads", "origin"]).await?;
        let mut heads: Vec<String> = out
            .lines()
            .filter_map(|l| l.split('\t').nth(1))
            .filter_map(|r| r.strip_prefix("refs/heads/"))
            .map(str::to_string)
            .collect();
        heads.sort();
        heads.dedup();
        Ok(heads)
    }

    /// Fast-forward only; a branch that moved on both sides is `Diverged`.
    pub async fn pull(&self) -> Result<()> {
        match self.text(&["pull", "--ff-only"]).await {
            Ok(_) => Ok(()),
            Err(Error::Failed { stderr, .. }) if stderr.contains("fast-forward") => {
                Err(Error::Diverged {
                    branch: self.current_branch().await.unwrap_or_default(),
                })
            }
            Err(e) => Err(e),
        }
    }

    /// The branch to its own name on origin, tracking set.
    pub async fn push(&self, branch: &str) -> Result<()> {
        let refspec = format!("{branch}:{branch}");
        self.text(&["push", "origin", &refspec, "--set-upstream"])
            .await
            .map(drop)
    }

    /// `git clone <url> <dest>`, run in the destination's parent.
    pub async fn clone(url: &str, dest: &Path) -> Result<Git> {
        let parent = dest.parent().unwrap_or(Path::new("."));
        let dest_str = dest.to_string_lossy();
        Git::at(parent).text(&["clone", url, &dest_str]).await?;
        Ok(Git::at(dest))
    }
}
