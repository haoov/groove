//! A revision a link asked for, and what it takes next: a worktree selected, its commit read.

use groove_types::{RepoId, SessionId, Worktree, WorktreeId};

/// A revision a link asked for: a worktree of `slug` in `session`, then `sha` shown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision {
    pub session: SessionId,
    pub slug: String,
    pub sha: Option<String>,
    /// Its commit's read is out; the revision ends when it lands.
    pub reading: bool,
}

/// What the revision asks next.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// Nothing yet: the worktree it needs is not read, or not there.
    Wait,
    /// It is over: another session is up, or there is no commit to show.
    Drop,
    /// This worktree selected first.
    Select(WorktreeId),
    /// This commit read and shown.
    Read(String),
}

impl Revision {
    /// Whether the repo `id` is the one it names, whole segments only.
    pub fn names(&self, id: &RepoId) -> bool {
        let (id, slug) = (id.as_str(), self.slug.as_str());
        id == slug || slug.ends_with(&format!("/{id}")) || id.ends_with(&format!("/{slug}"))
    }

    /// The next step, with `session` up, `selected` its worktree, read unless `stale`.
    pub fn step(
        &self,
        session: &SessionId,
        selected: Option<&Worktree>,
        worktrees: &[Worktree],
        stale: bool,
    ) -> Step {
        if *session != self.session {
            return Step::Drop;
        }
        if selected.is_some_and(|one| self.names(&one.repo)) {
            return match (&self.sha, stale) {
                (_, true) => Step::Wait,
                (Some(sha), false) => Step::Read(sha.clone()),
                (None, false) => Step::Drop,
            };
        }
        let ours = worktrees.iter().find(|one| self.names(&one.repo));
        ours.map_or(Step::Wait, |one| Step::Select(one.id.clone()))
    }
}
