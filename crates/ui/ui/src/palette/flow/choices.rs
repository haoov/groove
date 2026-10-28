//! What a prompt offers to pick: clones, sessions, repos, worktrees, and a base branch.

use groove_controllers::{AppState, session_service::Open};
use groove_types::RepoId;

use super::{CLONE, CLONE_LABEL, Prompt};

/// The pool's clones the session does not hold yet, then the clone row.
pub(super) fn pool_choices(app: &AppState, open: &Open) -> Vec<(String, String)> {
    app.session
        .pool
        .iter()
        .filter(|e| !open.repos.iter().any(|r| r.id.as_str() == e.slug))
        .map(|e| (e.slug.clone(), e.slug.clone()))
        .chain(std::iter::once((
            CLONE_LABEL.to_string(),
            CLONE.to_string(),
        )))
        .collect()
}

/// Every session on disk, open or not.
pub(super) fn session_choices(app: &AppState) -> Vec<(String, String)> {
    app.session
        .living
        .iter()
        .map(|one| {
            (
                format!("{} · {}", one.session.title, one.session.id),
                one.session.id.to_string(),
            )
        })
        .collect()
}

pub(super) fn repo_choices(open: &Open) -> Vec<(String, String)> {
    open.repos
        .iter()
        .map(|r| (format!("{} · {}", r.project, r.id), r.id.to_string()))
        .collect()
}

pub(super) fn worktree_choices(open: &Open) -> Vec<(String, String)> {
    open.worktrees
        .iter()
        .map(|w| {
            let project = open.repo_of(w).map_or("?", |r| r.project.as_str());
            (format!("{project} · {}", w.branch), w.id.to_string())
        })
        .collect()
}

/// Origin's heads as last listed; empty means the repo's default.
pub(super) fn base_prompt(app: &AppState, repo: &RepoId) -> Prompt {
    let heads = app
        .session
        .branches
        .iter()
        .find(|(r, _)| r == repo)
        .map(|(_, heads)| heads.iter().map(|h| (h.clone(), h.clone())).collect())
        .unwrap_or_default();
    Prompt {
        label: "target",
        options: heads,
        free: true,
        allow_empty: true,
        exact: false,
    }
}
