//! Attaching a repo to a task on the agent's behalf.
//!
//! The UI path is `set_task_repos` + `provision_worktrees`, driven by a picker
//! where the user has already chosen from a list. An agent has neither, so this
//! module answers the two questions the picker answered implicitly: WHICH repo did
//! it mean, and is attaching one to THIS session even meaningful.
//!
//! Attaching is additive here. `set_task_repos` deletes the whole set before
//! re-inserting, which is right for a picker that submits the full selection and
//! wrong for "add one more" — it would silently detach every other repo.

use sqlx::SqlitePool;
use tauri::Manager;

use crate::core::db::models::{Repo, SessionKind};
use crate::core::db::store;
use crate::worktrees::MainRepo;

/// Every name a slug answers to: itself, and each run of trailing segments. So
/// `gitlab.example.com/wiremind/devops/foo` is named by that, by
/// `wiremind/devops/foo`, by `devops/foo`, and by `foo`.
///
/// The host is a segment like any other, which is what keeps the forge path — the
/// name a person or an agent actually knows — working now that the pool puts the
/// host in front of it.
fn names_of(slug: &str) -> impl Iterator<Item = &str> {
    std::iter::once(slug).chain(slug.match_indices('/').map(|(i, _)| &slug[i + 1..]))
}

/// Pick the repo an agent meant from the clones in the pool.
///
/// The whole slug first, then any name it answers to. A name matching several slugs
/// is an error rather than a guess: attaching the wrong repo provisions a worktree
/// and a branch under it.
fn resolve<'a>(name: &str, available: &'a [MainRepo]) -> anyhow::Result<&'a MainRepo> {
    let wanted = name.trim().trim_matches('/');
    if wanted.is_empty() {
        anyhow::bail!("no repo name given");
    }
    let eq = |a: &str, b: &str| a.eq_ignore_ascii_case(b);

    if let Some(hit) = available.iter().find(|r| eq(&r.slug, wanted)) {
        return Ok(hit);
    }

    let matched: Vec<&MainRepo> = available
        .iter()
        .filter(|r| names_of(&r.slug).any(|n| eq(n, wanted)))
        .collect();
    match matched.as_slice() {
        [one] => Ok(one),
        [] => anyhow::bail!(
            "no repo named '{wanted}' is cloned in the pool. Available: {}. \
             Ask the user to clone it first — this tool cannot clone.",
            slug_list(available)
        ),
        many => anyhow::bail!(
            "'{wanted}' matches several repos: {}. Pass the full slug.",
            many.iter().map(|r| r.slug.as_str()).collect::<Vec<_>>().join(", ")
        ),
    }
}

fn slug_list(repos: &[MainRepo]) -> String {
    if repos.is_empty() {
        return "none".to_string();
    }
    repos.iter().map(|r| r.slug.as_str()).collect::<Vec<_>>().join(", ")
}

/// Whether this session can take a new repo. A review's worktree is the MR's
/// source branch; a second repo would get a worktree on a branch named after
/// the review, which is meaningless.
fn refusal_for(kind: SessionKind) -> Option<&'static str> {
    (kind == SessionKind::Review)
        .then_some("a review session tracks one MR and takes no extra repos")
}

/// Refuse a target branch that is not on origin.
async fn check_target(repo: &Repo, target: Option<&str>) -> anyhow::Result<()> {
    let Some(t) = target else { return Ok(()) };
    if !crate::core::git::refs::branch_on_remote(&repo.local_path, t).await {
        anyhow::bail!("{} has no branch '{t}' on origin — check the name", repo.project);
    }
    Ok(())
}

/// Attach `repo` to `task_id` and provision its worktree. The confirmation-bridge
/// path for `task.add_repo` — the user approves before any of this runs.
///
/// Re-opens the task at the end. Writing the rows is not enough: the workspace
/// holds its repos and worktrees in memory, so without the `workspace_ready` that
/// `open_task_impl` emits, an added repo stays invisible until the session is
/// reopened by hand. The UI's own add-repo modal invokes `open_task` for exactly
/// this reason; doing it here means every caller gets the refresh.
pub async fn add_repo_impl(
    payload: serde_json::Value,
    pool: &SqlitePool,
    app: &tauri::AppHandle,
) -> anyhow::Result<serde_json::Value> {
    let task_id = payload["task_id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing task_id"))?;
    let name = payload["repo"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing repo"))?;
    let branch = payload["branch"].as_str().filter(|s| !s.trim().is_empty());
    let target = payload["target_branch"].as_str().map(str::trim).filter(|s| !s.is_empty());

    let kind = store::sessions::kind_of(pool, task_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("no session {task_id}"))?;
    if let Some(why) = refusal_for(kind) {
        anyhow::bail!("{why}");
    }

    let available = crate::worktrees::list_main_repos()
        .await
        .map_err(|e| anyhow::anyhow!(e))?;
    let picked = resolve(name, &available)?;

    // Idempotent: a repo already registered keeps its id, so re-attaching is safe.
    let repo: Repo =
        crate::worktrees::register_repo_impl(&picked.slug, picked.local_path.clone(), pool)
            .await?;

    check_target(&repo, target).await?;
    store::repos::attach(pool, task_id, &repo.id).await?;

    let spec = crate::worktrees::BranchSpec {
        repo_id: repo.id.clone(),
        branch_name: branch.map(|b| b.to_string()),
        target_branch: target.map(|b| b.to_string()),
    };
    let worktrees = crate::worktrees::provision_worktrees_impl(task_id, &[spec], pool).await?;

    let wt = worktrees
        .first()
        .ok_or_else(|| anyhow::anyhow!("{} was attached but no worktree was created", repo.project))?
        .clone();

    // Push the new state to the workspace. Failing here does not undo the add —
    // the repo IS attached — so report it rather than turning a done job into an
    // error the agent will try to repeat.
    let task_state = app.state::<super::State>();
    if let Err(e) = super::open_task_impl(app, task_id, &task_state, pool).await {
        tracing::warn!("added {} to {task_id} but could not refresh the workspace: {e}", repo.project);
    }

    Ok(serde_json::json!({
        "repo": { "id": repo.id, "project": repo.project, "local_path": repo.local_path },
        "branch": wt.branch,
        "target_branch": wt.base_ref,
        "worktree_path": wt.path,
        "message": match &wt.base_ref {
            Some(base) => format!("Added {} to {task_id}, based on {base}", repo.project),
            None => format!("Added {} to {task_id}", repo.project),
        },
    }))
}

/// Add ANOTHER worktree for a repo the session already holds — the same repo, a
/// different branch. The confirmation-bridge path for `task.add_worktree`.
///
/// Separate from `add_repo_impl` on purpose: that one attaches a repo the session
/// does not have and may derive the branch, while this one requires the branch (a
/// second worktree with the same branch is the first one) and requires the repo to
/// be attached already. One op, one meaning — the approval dialog says which.
pub async fn add_worktree_impl(
    payload: serde_json::Value,
    pool: &SqlitePool,
    app: &tauri::AppHandle,
) -> anyhow::Result<serde_json::Value> {
    let task_id = payload["task_id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing task_id"))?;
    let branch = payload["branch"]
        .as_str()
        .map(str::trim)
        .filter(|b| !b.is_empty())
        .ok_or_else(|| anyhow::anyhow!("a branch name is required — that is what makes it a second worktree"))?;
    let target = payload["target_branch"].as_str().map(str::trim).filter(|b| !b.is_empty());

    let kind = store::sessions::kind_of(pool, task_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("no session {task_id}"))?;
    if let Some(why) = refusal_for(kind) {
        anyhow::bail!("{why}");
    }

    // The repo must already be on the session: attaching one is add_task_repo's job.
    let attached = store::repos::attached_to(pool, task_id).await?;
    let repo = match payload["repo"].as_str().map(str::trim).filter(|s| !s.is_empty()) {
        Some(name) => attached
            .iter()
            .find(|r| {
                r.project.eq_ignore_ascii_case(name)
                    || format!("{}/{}", r.group_path, r.project).eq_ignore_ascii_case(name)
            })
            .cloned()
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "{task_id} has no repo '{name}'. It has: {}. Use add_task_repo to attach a new one.",
                    attached.iter().map(|r| r.project.as_str()).collect::<Vec<_>>().join(", ")
                )
            })?,
        // One repo needs no naming; several do.
        None => match attached.as_slice() {
            [only] => only.clone(),
            [] => anyhow::bail!("{task_id} has no repos yet — use add_task_repo"),
            many => anyhow::bail!(
                "{task_id} has {} repos — say which: {}",
                many.len(),
                many.iter().map(|r| r.project.as_str()).collect::<Vec<_>>().join(", ")
            ),
        },
    };

    // A worktree already on this branch IS this request's outcome, so say so
    // rather than reporting a no-op as success.
    if let Some(existing) = store::worktrees::for_repo(pool, task_id, &repo.id)
        .await?
        .into_iter()
        .find(|wt| wt.branch == branch)
    {
        anyhow::bail!(
            "{} already has a worktree on {branch} at {}",
            repo.project,
            existing.path
        );
    }

    check_target(&repo, target).await?;

    let spec = crate::worktrees::BranchSpec {
        repo_id: repo.id.clone(),
        branch_name: Some(branch.to_string()),
        target_branch: target.map(|b| b.to_string()),
    };
    let worktrees = crate::worktrees::provision_worktrees_impl(task_id, &[spec], pool).await?;
    let wt = worktrees
        .first()
        .ok_or_else(|| anyhow::anyhow!("no worktree was created for {}", repo.project))?
        .clone();

    // Same reason as add_repo_impl: the workspace holds its worktrees in memory.
    let task_state = app.state::<super::State>();
    if let Err(e) = super::open_task_impl(app, task_id, &task_state, pool).await {
        tracing::warn!("added {branch} to {task_id} but could not refresh the workspace: {e}");
    }

    Ok(serde_json::json!({
        "repo": { "id": repo.id, "project": repo.project },
        "branch": wt.branch,
        "target_branch": wt.base_ref,
        "worktree_id": wt.id,
        "worktree_path": wt.path,
        "message": match &wt.base_ref {
            Some(base) => format!("Added a {} worktree on {branch}, based on {base}", repo.project),
            None => format!("Added a {} worktree on {branch}", repo.project),
        },
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `slug` is what the pool reports: host first (see worktrees::pool).
    fn main_repo(slug: &str) -> MainRepo {
        MainRepo {
            local_path: format!("/home/u/worktrees/main/{slug}"),
            slug: slug.to_string(),
        }
    }

    fn fixture() -> Vec<MainRepo> {
        vec![
            main_repo("gitlab.example.com/wiremind/devops/gitlab-ci-common"),
            main_repo("gitlab.example.com/wiremind/devops/testack-deploy"),
            main_repo("gitlab.example.com/wiremind/platform/testack-deploy"),
            main_repo("github.com/wiremind/wiremind-helm-charts"),
        ]
    }

    #[test]
    fn exact_slug_wins() {
        let repos = fixture();
        let hit = resolve("gitlab.example.com/wiremind/platform/testack-deploy", &repos).unwrap();
        assert_eq!(hit.slug, "gitlab.example.com/wiremind/platform/testack-deploy");
    }

    /// The forge path, which is the name anyone actually knows — and the one the
    /// ambiguity error tells an agent to pass. The pool prefixes the host, so this
    /// is no longer the whole slug and has to resolve all the same.
    #[test]
    fn the_forge_path_resolves_without_its_host() {
        let repos = fixture();
        assert_eq!(
            resolve("wiremind/platform/testack-deploy", &repos).unwrap().slug,
            "gitlab.example.com/wiremind/platform/testack-deploy"
        );
        assert_eq!(
            resolve("platform/testack-deploy", &repos).unwrap().slug,
            "gitlab.example.com/wiremind/platform/testack-deploy"
        );
    }

    #[test]
    fn unique_project_name_resolves() {
        let repos = fixture();
        assert_eq!(
            resolve("gitlab-ci-common", &repos).unwrap().slug,
            "gitlab.example.com/wiremind/devops/gitlab-ci-common"
        );
    }

    #[test]
    fn case_and_stray_slashes_are_tolerated() {
        let repos = fixture();
        assert_eq!(
            resolve("/GitLab-CI-Common/", &repos).unwrap().slug,
            "gitlab.example.com/wiremind/devops/gitlab-ci-common"
        );
    }

    /// The important failure: two repos share a project name, so guessing would
    /// provision a branch on the wrong one.
    #[test]
    fn ambiguous_project_name_is_refused() {
        let repos = fixture();
        let err = resolve("testack-deploy", &repos).unwrap_err().to_string();
        assert!(err.contains("matches several"), "{err}");
        assert!(err.contains("gitlab.example.com/wiremind/devops/testack-deploy"), "{err}");
        assert!(err.contains("gitlab.example.com/wiremind/platform/testack-deploy"), "{err}");
    }

    #[test]
    fn unknown_repo_says_to_clone_it() {
        let repos = fixture();
        let err = resolve("nope", &repos).unwrap_err().to_string();
        assert!(err.contains("cannot clone"), "{err}");
        assert!(err.contains("gitlab-ci-common"), "{err}");
    }

    #[test]
    fn empty_name_is_refused() {
        assert!(resolve("  ", &fixture()).is_err());
    }

    #[test]
    fn only_review_sessions_refuse_new_repos() {
        assert!(refusal_for(SessionKind::Review).is_some());
        assert!(refusal_for(SessionKind::Explorer).is_none());
        assert!(refusal_for(SessionKind::Task).is_none());
    }
}
