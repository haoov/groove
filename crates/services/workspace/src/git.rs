//! What git is asked for on the selected worktree.

use std::path::Path;

use groove_types::Result;

/// What the index holds, and the commit that turns it into history.
pub async fn stage(dir: &Path, paths: &[String]) -> Result<()> {
    groove_git::Git::at(dir).stage(paths).await?;
    Ok(())
}

pub async fn unstage(dir: &Path, paths: &[String]) -> Result<()> {
    groove_git::Git::at(dir).unstage(paths).await?;
    Ok(())
}

pub async fn discard(dir: &Path, paths: &[String]) -> Result<()> {
    groove_git::Git::at(dir).discard(paths).await?;
    Ok(())
}

pub async fn commit(dir: &Path, message: &str) -> Result<()> {
    groove_git::Git::at(dir).commit(message).await?;
    Ok(())
}

/// What the branch does against its remote and its base.
pub async fn push(dir: &Path, branch: &str) -> Result<()> {
    groove_git::Git::at(dir).push(branch).await?;
    Ok(())
}

/// The commits a push would send: over the branch on origin, or over its base when origin has none.
pub async fn unpushed(
    dir: &Path,
    branch: &str,
    base: Option<&str>,
) -> Result<Vec<groove_types::CommitEntry>> {
    let git = groove_git::Git::at(dir);
    let upstream = format!("origin/{branch}");
    let against = match (git.ref_exists(&upstream).await?, base) {
        (true, _) => upstream,
        (false, Some(base)) => base.to_string(),
        (false, None) => git.base_ref(None).await?,
    };
    let log = git.log(Some(&against), UNPUSHED_MAX).await?;
    Ok(log.into_iter().filter(|one| !one.is_base).collect())
}

/// How many of them a push ask lists.
pub const UNPUSHED_MAX: usize = 20;

pub async fn pull(dir: &Path) -> Result<()> {
    groove_git::Git::at(dir).pull().await?;
    Ok(())
}
