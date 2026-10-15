//! The header pickers' choices: a repo by its worktrees, or a worktree of the repo in hand.

use groove_controllers::session_service::Open;

/// Each repo holding a worktree, to its first one.
pub(super) fn repos(open: &Open) -> Vec<(String, String)> {
    let first = |repo: &groove_types::RepoId| open.worktrees.iter().find(|w| &w.repo == repo);
    open.repos
        .iter()
        .filter_map(|repo| Some((repo.project.clone(), first(&repo.id)?.id.to_string())))
        .collect()
}

/// The selected repo's worktrees, by branch.
pub(super) fn branches(open: &Open) -> Vec<(String, String)> {
    let Some(repo) = open.selected_repo() else {
        return Vec::new();
    };
    let held = open.worktrees.iter().filter(|w| w.repo == repo.id);
    held.map(|w| (w.branch.clone(), w.id.to_string())).collect()
}
