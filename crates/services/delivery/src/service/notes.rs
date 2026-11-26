//! What the session says on the forge's threads.

use groove_forge::{Posted, Verdict};
use groove_types::{Anchor, Repo, Result, ReviewVerdict, WorktreeId};

use super::Service;

/// What a review says: its verdict and its words.
pub struct Said<'a> {
    pub verdict: ReviewVerdict,
    pub body: &'a str,
}

impl Said<'_> {
    /// Why a comment has nothing to post; a review posts as it stands, words or none.
    pub fn empty(verdict: Option<ReviewVerdict>, body: &str) -> Result<()> {
        match (verdict, body.is_empty()) {
            (None, true) => Err(groove_types::Error::invalid("a comment needs a body")),
            _ => Ok(()),
        }
    }
}

impl Service {
    /// A new thread of the MR on lines of a file's new side.
    pub async fn comment_lines(
        &self,
        repo: &Repo,
        worktree: &WorktreeId,
        anchor: &Anchor,
        body: &str,
    ) -> Result<()> {
        let (remote, mr) = (self.remote(repo)?, self.held(worktree).await?);
        let posted = Posted {
            path: &anchor.path,
            from: anchor.start_line + 1,
            to: anchor.end_line + 1,
            body,
        };
        Ok(remote.post_note(repo, &mr.remote_id, posted).await?)
    }

    /// Words under a thread of the MR.
    pub async fn reply_thread(
        &self,
        repo: &Repo,
        worktree: &WorktreeId,
        thread: &str,
        body: &str,
    ) -> Result<()> {
        let (remote, mr) = (self.remote(repo)?, self.held(worktree).await?);
        Ok(remote
            .reply_thread(repo, &mr.remote_id, thread, body)
            .await?)
    }

    /// A comment on the MR itself, under no line.
    pub async fn comment(&self, repo: &Repo, worktree: &WorktreeId, body: &str) -> Result<()> {
        let (remote, mr) = (self.remote(repo)?, self.held(worktree).await?);
        Ok(remote.comment(repo, &mr.remote_id, body).await?)
    }

    /// A verdict on the MR, with its words.
    pub async fn review(&self, repo: &Repo, worktree: &WorktreeId, said: Said<'_>) -> Result<()> {
        let (remote, mr) = (self.remote(repo)?, self.held(worktree).await?);
        let verdict = Verdict {
            said: said.verdict,
            body: said.body,
            notes: &[],
        };
        Ok(remote.review(repo, &mr.remote_id, verdict).await?)
    }

    /// A thread of the MR resolved, or opened again.
    pub async fn resolve_thread(&self, repo: &Repo, thread: &str, resolve: bool) -> Result<()> {
        Ok(self.remote(repo)?.resolve_thread(thread, resolve).await?)
    }
}
