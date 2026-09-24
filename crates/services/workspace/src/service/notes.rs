//! What the capability asks of the notes: this session's own, and the forge's threads.

use groove_annotations::New;
use groove_forge::{Posted, Remote, Verdict};
use groove_types::{
    Annotation, AnnotationId, Forge, Repo, Result, ReviewVerdict, SessionId, Timestamp, WorktreeId,
};

use super::Service;

/// What a review says: its verdict, its words, and the notes it carries.
pub struct Said<'a> {
    pub verdict: ReviewVerdict,
    pub body: &'a str,
    pub notes: &'a [Annotation],
}

/// A note of another repo has no line on this merge request.
fn elsewhere(note: &Annotation, repo: &Repo) -> Result<()> {
    match note.repo == repo.id {
        true => Ok(()),
        false => Err(groove_types::Error::invalid(format!(
            "{} is a note of {}, not of {}",
            note.id, note.repo, repo.id
        ))),
    }
}

/// One note of this session as the forges take it, lines and all.
pub(crate) fn posted(note: &Annotation) -> Posted<'_> {
    Posted {
        path: &note.file_path,
        from: note.start_line + 1,
        to: note.end_line + 1,
        body: &note.content,
    }
}

impl Service {
    /// The notes this session left, whichever file they stand on.
    pub async fn notes(&self, session: &SessionId) -> Result<Vec<Annotation>> {
        self.notes.list(session).await
    }

    pub async fn create_note(&self, new: New, now: Timestamp) -> Result<Annotation> {
        self.notes.create(new, now).await
    }

    pub async fn update_note(&self, id: &AnnotationId, content: &str) -> Result<Annotation> {
        self.notes.update(id, content).await
    }

    pub async fn resolve_note(&self, id: &AnnotationId) -> Result<Annotation> {
        self.notes.resolve(id).await
    }

    pub async fn reopen_note(&self, id: &AnnotationId) -> Result<Annotation> {
        self.notes.reopen(id).await
    }

    pub async fn delete_note(&self, id: &AnnotationId) -> Result<()> {
        self.notes.delete(id).await
    }

    /// One note of this session posted on the MR, and resolved here once it is there.
    pub async fn post_note(
        &self,
        remote: &Remote,
        repo: &Repo,
        worktree: &WorktreeId,
        note: &Annotation,
    ) -> Result<()> {
        elsewhere(note, repo)?;
        let mr = self.held(worktree).await?;
        remote.post_note(repo, &mr.remote_id, posted(note)).await?;
        self.notes.delete(&note.id).await
    }

    /// Words under a thread of the MR.
    pub async fn reply_thread(
        &self,
        remote: &Remote,
        repo: &Repo,
        worktree: &WorktreeId,
        thread: &str,
        body: &str,
    ) -> Result<()> {
        let mr = self.held(worktree).await?;
        Ok(remote
            .reply_thread(repo, &mr.remote_id, thread, body)
            .await?)
    }

    /// A comment on the MR itself, under no line.
    pub async fn comment(
        &self,
        remote: &Remote,
        repo: &Repo,
        worktree: &WorktreeId,
        body: &str,
    ) -> Result<()> {
        let mr = self.held(worktree).await?;
        Ok(remote.comment(repo, &mr.remote_id, body).await?)
    }

    /// A verdict on the MR, carrying the notes this session has not posted, which
    /// it resolves once they are up.
    pub async fn review(
        &self,
        remote: &Remote,
        repo: &Repo,
        worktree: &WorktreeId,
        said: Said<'_>,
    ) -> Result<()> {
        match remote.kind() {
            Forge::Github => self.reviewed(remote, repo, worktree, said).await,
            Forge::Gitlab => self.one_by_one(remote, repo, worktree, said).await,
        }
    }

    /// One call carries the verdict and every note, so all of them land or none do.
    async fn reviewed(
        &self,
        remote: &Remote,
        repo: &Repo,
        worktree: &WorktreeId,
        said: Said<'_>,
    ) -> Result<()> {
        let mr = self.held(worktree).await?;
        let notes: Vec<Posted<'_>> = said.notes.iter().map(posted).collect();
        let verdict = Verdict {
            said: said.verdict,
            body: said.body,
            notes: &notes,
        };
        remote.review(repo, &mr.remote_id, verdict).await?;
        for note in said.notes {
            self.notes.delete(&note.id).await?;
        }
        Ok(())
    }

    /// A note at a time, each taken away as it lands, then the verdict on its own: a
    /// call that fails leaves nothing to post twice.
    async fn one_by_one(
        &self,
        remote: &Remote,
        repo: &Repo,
        worktree: &WorktreeId,
        said: Said<'_>,
    ) -> Result<()> {
        for note in said.notes {
            self.post_note(remote, repo, worktree, note).await?;
        }
        let mr = self.held(worktree).await?;
        let verdict = Verdict {
            said: said.verdict,
            body: said.body,
            notes: &[],
        };
        Ok(remote.review(repo, &mr.remote_id, verdict).await?)
    }

    /// A thread of the MR resolved, or opened again.
    pub async fn resolve_thread(&self, remote: &Remote, thread: &str, resolve: bool) -> Result<()> {
        Ok(remote.resolve_thread(thread, resolve).await?)
    }
}
