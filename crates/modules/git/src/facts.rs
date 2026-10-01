//! What a branch is against its remote and its base: ahead, behind, the fork point.

use crate::parse::RemoteUrl;
use crate::{Error, Git, Result};

/// Where a branch forks from when nothing is pinned, in order.
const BASE_CANDIDATES: [&str; 3] = ["origin/HEAD", "origin/main", "origin/master"];

impl Git {
    pub async fn version(&self) -> Result<String> {
        self.line(&["--version"]).await
    }

    /// Where git keeps this worktree's own HEAD and index.
    pub async fn git_dir(&self) -> Result<std::path::PathBuf> {
        let out = self.line(&["rev-parse", "--absolute-git-dir"]).await?;
        Ok(std::path::PathBuf::from(out))
    }

    pub async fn is_repository(&self) -> Result<bool> {
        self.succeeds(&["rev-parse", "--git-dir"]).await
    }

    /// Whether the remote is configured, whatever its URL looks like.
    pub async fn has_remote(&self, remote: &str) -> Result<bool> {
        self.succeeds(&["remote", "get-url", remote]).await
    }

    pub async fn remote_url(&self, remote: &str) -> Result<RemoteUrl> {
        let url = self.line(&["remote", "get-url", remote]).await?;
        RemoteUrl::parse(&url)
    }

    pub async fn current_branch(&self) -> Result<String> {
        self.line(&["rev-parse", "--abbrev-ref", "HEAD"]).await
    }

    pub async fn ref_exists(&self, git_ref: &str) -> Result<bool> {
        self.succeeds(&["rev-parse", "--verify", "--quiet", git_ref])
            .await
    }

    /// Where origin's copy of the work stands: the branch there, else its base.
    pub async fn pushed_point(&self, branch: &str, pinned: Option<&str>) -> Result<String> {
        match self.remote_of(branch).await? {
            Some(remote) => Ok(remote),
            None => self.base_ref(pinned).await,
        }
    }

    /// `origin/<branch>`, when it exists and the branch tracks nothing else.
    pub async fn remote_of(&self, branch: &str) -> Result<Option<String>> {
        let named = format!("origin/{branch}");
        if !self.ref_exists(&named).await? {
            return Ok(None);
        }
        let tracked = format!("{branch}@{{upstream}}");
        let upstream = self
            .line(&["rev-parse", "--abbrev-ref", &tracked])
            .await
            .ok();
        Ok(match upstream {
            Some(other) if other != named => None,
            _ => Some(named),
        })
    }

    pub async fn merge_base(&self, a: &str, b: &str) -> Result<String> {
        self.line(&["merge-base", a, b]).await
    }

    /// The remote's default branch: the local `origin/HEAD` symref, else asked of origin.
    pub async fn default_branch(&self) -> Result<Option<String>> {
        if let Some(name) = self.local_default_branch().await {
            return Ok(Some(name));
        }
        self.remote_default_branch().await
    }

    async fn local_default_branch(&self) -> Option<String> {
        let symref = self
            .line(&["symbolic-ref", "--short", "refs/remotes/origin/HEAD"])
            .await
            .ok()?;
        let name = symref.strip_prefix("origin/")?;
        (!name.is_empty() && name != "HEAD").then(|| name.to_string())
    }

    /// `ls-remote --symref origin HEAD` prints `ref: refs/heads/<name>\tHEAD` first.
    async fn remote_default_branch(&self) -> Result<Option<String>> {
        let out = self
            .text(&["ls-remote", "--symref", "origin", "HEAD"])
            .await?;
        Ok(out
            .lines()
            .find_map(|l| l.strip_prefix("ref: refs/heads/"))
            .and_then(|rest| rest.split('\t').next())
            .filter(|name| !name.is_empty())
            .map(str::to_string))
    }

    /// The ref this work forks from: the pinned target on origin, else the usual names.
    pub async fn base_ref(&self, pinned: Option<&str>) -> Result<String> {
        let mut tried = Vec::new();
        for candidate in pinned
            .filter(|b| !b.is_empty())
            .map(|b| format!("origin/{b}"))
            .into_iter()
            .chain(BASE_CANDIDATES.iter().map(|c| c.to_string()))
        {
            if self.ref_exists(&candidate).await? {
                return Ok(candidate);
            }
            tried.push(candidate);
        }
        Err(Error::NoBase {
            dir: self.dir.clone(),
            tried,
        })
    }
}
