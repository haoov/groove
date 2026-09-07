//! Attaching a repo or a worktree to a session on the agent's behalf.
//! Attaching is additive here; `set_task_repos` replaces the whole set.

use sqlx::SqlitePool;
use tauri::Manager;

use crate::core::db::models::{Repo, SessionKind};
use crate::core::db::store;
use crate::worktrees::MainRepo;

/// Every name a slug answers to: `host/a/b` is named by `host/a/b`, `a/b` and `b`.
fn names_of(slug: &str) -> impl Iterator<Item = &str> {
    std::iter::once(slug).chain(slug.match_indices('/').map(|(i, _)| &slug[i + 1..]))
}

/// Pick the repo an agent meant: the whole slug first, then a unique short name.
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
            many.iter()
                .map(|r| r.slug.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn slug_list(repos: &[MainRepo]) -> String {
    if repos.is_empty() {
        return "none".to_string();
    }
    repos
        .iter()
        .map(|r| r.slug.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Why this session refuses a new repo, if it does.
fn refusal_for(kind: SessionKind) -> Option<&'static str> {
    (kind == SessionKind::Review)
        .then_some("a review session tracks one MR and takes no extra repos")
}

/// Branch names listed in the refusal before it falls back to a count.
const TARGET_SUGGESTIONS: usize = 20;

/// Refuse a target branch origin does not have, naming the ones it does.
async fn check_target(repo: &Repo, target: Option<&str>) -> anyhow::Result<()> {
    let Some(t) = target else { return Ok(()) };

    // An unreachable origin is not a missing branch.
    let branches = crate::core::git::refs::origin_branches(&repo.local_path)
        .await
        .map_err(|e| anyhow::anyhow!("cannot reach origin for {}: {e}", repo.project))?;

    if branches.iter().any(|b| b == t) {
        return Ok(());
    }

    let shown: Vec<&str> = branches
        .iter()
        .take(TARGET_SUGGESTIONS)
        .map(String::as_str)
        .collect();
    let rest = branches.len().saturating_sub(shown.len());
    let listed = match (shown.is_empty(), rest) {
        (true, _) => "it has none".to_string(),
        (false, 0) => format!("it has: {}", shown.join(", ")),
        (false, n) => format!("it has: {}, and {n} more", shown.join(", ")),
    };
    anyhow::bail!("{} has no branch '{t}' on origin — {listed}", repo.project);
}

/// Attach `repo` to `task_id`, provision its worktree, and refresh the workspace.
/// Bridge path for `task.add_repo`.
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
    let target = payload["target_branch"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty());

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

    let repo: Repo =
        crate::worktrees::register_repo_impl(&picked.slug, picked.local_path.clone(), pool).await?;

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
        .ok_or_else(|| {
            anyhow::anyhow!("{} was attached but no worktree was created", repo.project)
        })?
        .clone();

    // A failed refresh does not undo the add.
    let task_state = app.state::<super::State>();
    if let Err(e) = super::open_task_impl(
        app,
        task_id,
        &task_state,
        pool,
        super::commands::Open::Refresh,
    )
    .await
    {
        tracing::warn!(
            "added {} to {task_id} but could not refresh the workspace: {e}",
            repo.project
        );
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

/// Add a worktree on another branch for a repo the session already holds.
/// Bridge path for `task.add_worktree`.
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
        .ok_or_else(|| {
            anyhow::anyhow!("a branch name is required — that is what makes it a second worktree")
        })?;
    let target = payload["target_branch"]
        .as_str()
        .map(str::trim)
        .filter(|b| !b.is_empty());

    let kind = store::sessions::kind_of(pool, task_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("no session {task_id}"))?;
    if let Some(why) = refusal_for(kind) {
        anyhow::bail!("{why}");
    }

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

    let task_state = app.state::<super::State>();
    if let Err(e) = super::open_task_impl(
        app,
        task_id,
        &task_state,
        pool,
        super::commands::Open::Refresh,
    )
    .await
    {
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

    use std::path::PathBuf;

    /// A clone whose origin has `main` and `release/1.0`.
    struct Origin {
        root: PathBuf,
        repo: Repo,
    }

    impl Drop for Origin {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    /// Spawn git through `core::git`; a guard test in `core/git/run.rs` rejects any other path.
    async fn git(dir: &std::path::Path, args: &[&str]) {
        let mut full = vec![
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=T",
            "-c",
            "commit.gpgsign=false",
        ];
        full.extend_from_slice(args);
        crate::core::git::run(&dir.to_string_lossy(), &full)
            .await
            .unwrap_or_else(|e| panic!("git {args:?}: {e}"));
    }

    impl Origin {
        async fn new(name: &str) -> Self {
            let root =
                std::env::temp_dir().join(format!("groove-target-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(&root).unwrap();

            let origin = root.join("origin.git");
            std::fs::create_dir_all(&origin).unwrap();
            git(&origin, &["init", "--bare", "--initial-branch=main", "."]).await;

            let work = root.join("work");
            std::fs::create_dir_all(&work).unwrap();
            git(&work, &["init", "--initial-branch=main", "."]).await;
            std::fs::write(work.join("a.txt"), "one\n").unwrap();
            git(&work, &["add", "."]).await;
            git(&work, &["commit", "-m", "first"]).await;
            git(
                &work,
                &["remote", "add", "origin", origin.to_str().unwrap()],
            )
            .await;
            git(&work, &["push", "origin", "main"]).await;
            git(&work, &["push", "origin", "main:release/1.0"]).await;
            git(&work, &["fetch", "origin"]).await;

            let repo = Repo {
                id: "r1".into(),
                host: "example.com".into(),
                group_path: "g".into(),
                project: "proj".into(),
                local_path: work.to_string_lossy().to_string(),
            };
            Origin { root, repo }
        }
    }

    #[tokio::test]
    async fn no_target_is_always_fine() {
        let fx = Origin::new("none").await;
        check_target(&fx.repo, None).await.unwrap();
    }

    #[tokio::test]
    async fn a_branch_origin_has_passes() {
        let fx = Origin::new("has").await;
        check_target(&fx.repo, Some("release/1.0")).await.unwrap();
    }

    #[tokio::test]
    async fn a_missing_branch_is_refused_with_the_real_ones() {
        let fx = Origin::new("missing").await;
        let err = check_target(&fx.repo, Some("nope"))
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("has no branch 'nope'"), "{err}");
        assert!(
            err.contains("release/1.0"),
            "the refusal must list what origin has: {err}"
        );
    }

    #[tokio::test]
    async fn a_branch_deleted_on_origin_is_refused() {
        let fx = Origin::new("stale").await;
        let work = PathBuf::from(&fx.repo.local_path);
        git(&work, &["push", "origin", "--delete", "release/1.0"]).await;
        git(
            &work,
            &["update-ref", "refs/remotes/origin/release/1.0", "HEAD"],
        )
        .await;
        crate::core::git::cache::flush();
        let err = check_target(&fx.repo, Some("release/1.0"))
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("has no branch"), "{err}");
    }

    #[tokio::test]
    async fn an_unreachable_origin_says_so() {
        let fx = Origin::new("unreachable").await;
        let work = PathBuf::from(&fx.repo.local_path);
        git(
            &work,
            &["remote", "set-url", "origin", "/nonexistent/origin.git"],
        )
        .await;
        let err = check_target(&fx.repo, Some("main"))
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("cannot reach origin"), "{err}");
        assert!(!err.contains("has no branch"), "{err}");
    }

    /// A pool slug: host first.
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
        let hit = resolve(
            "gitlab.example.com/wiremind/platform/testack-deploy",
            &repos,
        )
        .unwrap();
        assert_eq!(
            hit.slug,
            "gitlab.example.com/wiremind/platform/testack-deploy"
        );
    }

    #[test]
    fn the_forge_path_resolves_without_its_host() {
        let repos = fixture();
        assert_eq!(
            resolve("wiremind/platform/testack-deploy", &repos)
                .unwrap()
                .slug,
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

    #[test]
    fn ambiguous_project_name_is_refused() {
        let repos = fixture();
        let err = resolve("testack-deploy", &repos).unwrap_err().to_string();
        assert!(err.contains("matches several"), "{err}");
        assert!(
            err.contains("gitlab.example.com/wiremind/devops/testack-deploy"),
            "{err}"
        );
        assert!(
            err.contains("gitlab.example.com/wiremind/platform/testack-deploy"),
            "{err}"
        );
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
