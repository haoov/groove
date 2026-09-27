//! What the index holds, and the commit that turns it into history.

use crate::{Git, Result};

impl Git {
    /// Puts `paths` in the index: a change, a new file or a removal.
    pub async fn stage(&self, paths: &[String]) -> Result<()> {
        self.on(&["add", "--"], paths).await
    }

    /// Takes `paths` out of the index and leaves the working tree alone.
    pub async fn unstage(&self, paths: &[String]) -> Result<()> {
        self.on(&["restore", "--staged", "--"], paths).await
    }

    /// Throws away what `paths` hold: back to HEAD, and what is untracked removed.
    pub async fn discard(&self, paths: &[String]) -> Result<()> {
        let committed = self.in_head(paths).await?;
        let indexed = self.names(&["ls-files", "-z", "--"], paths).await?;
        self.on(&["restore", "--staged", "--"], &indexed).await?;
        self.on(&["restore", "--worktree", "--"], &committed)
            .await?;
        self.on(&["clean", "-fd", "--"], paths).await
    }

    /// Which of `paths` the last commit has. None of them before the first commit.
    async fn in_head(&self, paths: &[String]) -> Result<Vec<String>> {
        if !self.ref_exists("HEAD").await? {
            return Ok(Vec::new());
        }
        self.names(&["ls-tree", "-r", "--name-only", "-z", "HEAD", "--"], paths)
            .await
    }

    /// The paths `args` lists, of those given.
    async fn names(&self, args: &[&str], paths: &[String]) -> Result<Vec<String>> {
        if paths.is_empty() {
            return Ok(Vec::new());
        }
        let mut all: Vec<&str> = args.to_vec();
        all.extend(paths.iter().map(String::as_str));
        let out = self.text(&all).await?;
        Ok(out
            .split('\0')
            .filter(|name| !name.is_empty())
            .map(str::to_string)
            .collect())
    }

    /// Commits the index. Nothing staged is an error, as git says it is.
    pub async fn commit(&self, message: &str) -> Result<()> {
        self.text(&["commit", "-m", message]).await.map(drop)
    }

    /// `git <args> <paths>`, and nothing at all when there are no paths.
    async fn on(&self, args: &[&str], paths: &[String]) -> Result<()> {
        if paths.is_empty() {
            return Ok(());
        }
        let mut all: Vec<&str> = args.to_vec();
        all.extend(paths.iter().map(String::as_str));
        self.text(&all).await.map(drop)
    }
}
