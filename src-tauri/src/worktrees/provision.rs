//! One provisioning path for every session kind. A worktree lives at `<session>/<project>/<branch>`.

use serde::Deserialize;
use sqlx::SqlitePool;

use super::naming;
use super::pool::session_dir;
use crate::core::db::models::{Repo, Session, Worktree};
use crate::core::db::store;
use crate::core::error::AppResult;
use crate::core::git;

#[derive(Debug, Clone, Deserialize)]
pub struct BranchSpec {
    pub repo_id: String,
    /// None → the session-kind default from `naming::default_branch`.
    pub branch_name: Option<String>,
    /// Branch to cut from and merge back into. None → the repo default.
    #[serde(default)]
    pub target_branch: Option<String>,
}

#[tauri::command]
pub async fn provision_worktrees(
    task_id: String,
    branches: Vec<BranchSpec>,
    pool: tauri::State<'_, SqlitePool>,
) -> AppResult<Vec<Worktree>> {
    Ok(provision_worktrees_impl(&task_id, &branches, &pool).await?)
}

/// The branch a new worktree gets when the caller names none.
pub(crate) async fn default_branch_for(
    session_id: &str,
    pool: &SqlitePool,
) -> anyhow::Result<String> {
    let session = store::sessions::get(pool, session_id).await?;
    let tag = branch_tag_of(&session.id, pool).await;
    Ok(naming::default_branch(&session, tag.as_deref()))
}

/// What the task calls itself at its source, appended to the branch.
async fn branch_tag_of(session_id: &str, pool: &SqlitePool) -> Option<String> {
    store::provider_tasks::get_by_short_id(pool, session_id)
        .await
        .ok()
        .flatten()
        .and_then(|t| t.branch_tag)
}

/// The default branch for a session's next worktree, shown before anything is created.
#[tauri::command]
pub async fn default_branch_for_session(
    short_id: String,
    pool: tauri::State<'_, SqlitePool>,
) -> AppResult<String> {
    Ok(default_branch_for(&short_id, &pool).await?)
}

/// Provision one worktree per spec, concurrently.
pub(crate) async fn provision_worktrees_impl(
    session_id: &str,
    branches: &[BranchSpec],
    pool: &SqlitePool,
) -> anyhow::Result<Vec<Worktree>> {
    let session = store::sessions::get(pool, session_id).await?;

    let tag = branch_tag_of(&session.id, pool).await;

    futures_util::future::try_join_all(branches.iter().map(|spec| {
        let session = &session;
        let tag = tag.as_deref();
        async move {
            let repo = store::repos::get(pool, &spec.repo_id).await?;
            let asked = spec.branch_name.clone().filter(|b| !b.trim().is_empty());
            let branch = asked
                .clone()
                .unwrap_or_else(|| naming::default_branch(session, tag));
            // A typed name is a deliberate choice; a derived one must name this session.
            let own = asked.is_some() || names_session(&branch, session, tag);
            provision_one(
                pool,
                session,
                &repo,
                &branch,
                None,
                spec.target_branch.as_deref(),
                own,
            )
            .await
        }
    }))
    .await
}

/// Provision a review worktree: the MR's source branch, with its target as `base_ref`.
pub(crate) async fn provision_review_worktree(
    session_id: &str,
    repo: &Repo,
    source_branch: &str,
    target_branch: &str,
    pool: &SqlitePool,
) -> anyhow::Result<Worktree> {
    let session = store::sessions::get(pool, session_id).await?;

    // The default fetch may not cover the MR branches.
    let out = git::output(
        &repo.local_path,
        &["fetch", "origin", source_branch, target_branch],
    )
    .await?;
    git::cache::flush();
    if !out.status.success() {
        return Err(anyhow::anyhow!(
            "git fetch origin {source_branch} {target_branch} failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }

    // The MR's own source branch is the target here.
    let wt = provision_one(
        pool,
        &session,
        repo,
        source_branch,
        Some(source_branch),
        None,
        true,
    )
    .await?;
    store::worktrees::set_base_ref(pool, &wt.id, target_branch).await?;
    Ok(Worktree {
        base_ref: Some(target_branch.to_string()),
        ..wt
    })
}

/// Characters `git check-ref-format` refuses outright.
const FORBIDDEN_CHARS: &str = " ~^:?*[\\";

/// Reject a branch name git would refuse, before it can name a directory.
pub(crate) fn validate_branch_name(branch: &str) -> anyhow::Result<()> {
    let bad = |reason: &str| anyhow::anyhow!("invalid branch name '{branch}': {reason}");

    if branch.is_empty() {
        return Err(bad("empty"));
    }
    if branch.starts_with('/') || branch.ends_with('/') {
        return Err(bad("leading or trailing '/'"));
    }
    if branch.contains("//") {
        return Err(bad("empty path component"));
    }
    if branch.starts_with('-') {
        return Err(bad("leading '-'"));
    }
    if branch.contains("..") {
        return Err(bad("'..'"));
    }
    if let Some(c) = branch
        .chars()
        .find(|c| c.is_ascii_control() || FORBIDDEN_CHARS.contains(*c))
    {
        return Err(bad(&format!("forbidden character {c:?}")));
    }
    for component in branch.split('/') {
        if component.starts_with('.') {
            return Err(bad("component starting with '.'"));
        }
        if component.ends_with(".lock") {
            return Err(bad("component ending in '.lock'"));
        }
        if component.ends_with('.') {
            return Err(bad("component ending in '.'"));
        }
    }
    Ok(())
}

/// Fetch the clone, create the branch if needed, add the worktree, record it.
/// Idempotent: an existing worktree for this branch is reused wherever its directory sits.
async fn provision_one(
    pool: &SqlitePool,
    session: &Session,
    repo: &Repo,
    branch: &str,
    track_remote: Option<&str>,
    target: Option<&str>,
    branch_is_own: bool,
) -> anyhow::Result<Worktree> {
    validate_branch_name(branch)?;

    if let Some(existing) = existing_worktree(pool, session, repo, branch).await? {
        align_checkout(&existing.path, branch).await?;
        return Ok(existing);
    }

    if !git::run::is_repository(&repo.local_path).await {
        return Err(anyhow::anyhow!("Cannot open repo {}", repo.local_path));
    }

    // A deleted folder leaves a "missing but already registered" worktree behind.
    let _ = git::run(&repo.local_path, &["worktree", "prune"]).await;

    let repo_default = refresh_main_clone(&repo.local_path, &repo.project, &session.id).await;
    let branch_point = match target {
        Some(t) => BranchPoint::Exact(t),
        None => BranchPoint::RepoDefault(repo_default.as_deref()),
    };

    let wt_path = session_dir(&session.id).join(naming::worktree_dir(&repo.project, branch));
    std::fs::create_dir_all(&wt_path)?;
    let wt_path_str = wt_path.to_string_lossy().to_string();

    let local_exists =
        git::refs::ref_exists(&repo.local_path, &format!("refs/heads/{branch}")).await;
    let output = match track_remote {
        // Review: a branch not yet local tracks its origin counterpart.
        Some(remote_branch) if !local_exists => {
            let track = format!("origin/{remote_branch}");
            git::output(
                &repo.local_path,
                &[
                    "worktree",
                    "add",
                    "--track",
                    "-b",
                    branch,
                    &wt_path_str,
                    &track,
                ],
            )
            .await?
        }
        _ => {
            if local_exists {
                refuse_foreign_branch(&repo.local_path, &repo.project, branch, branch_is_own)
                    .await?;
                report_adopted_branch(&repo.local_path, &repo.project, branch, &session.id).await;
            } else {
                create_branch(branch, &repo.local_path, branch_point).await?;
            }
            git::output(&repo.local_path, &["worktree", "add", &wt_path_str, branch]).await?
        }
    };
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.contains("already exists") {
            return Err(anyhow::anyhow!("git worktree add failed: {stderr}"));
        }
        align_checkout(&wt_path_str, branch).await?;
    }

    git::cache::flush();
    let wt = store::worktrees::upsert(pool, &session.id, &repo.id, branch, &wt_path_str).await?;

    if let Some(t) = target {
        store::worktrees::set_base_ref(pool, &wt.id, t).await?;
        return Ok(Worktree {
            base_ref: Some(t.to_string()),
            ..wt
        });
    }
    Ok(wt)
}

/// The session's existing worktree for `(repo, branch)` whose directory is still on disk.
async fn existing_worktree(
    pool: &SqlitePool,
    session: &Session,
    repo: &Repo,
    branch: &str,
) -> anyhow::Result<Option<Worktree>> {
    Ok(store::worktrees::for_repo(pool, &session.id, &repo.id)
        .await?
        .into_iter()
        .find(|wt| wt.branch == branch && std::path::Path::new(&wt.path).is_dir()))
}

/// Switch the checkout to `branch` when it sits on another one.
async fn align_checkout(wt_path: &str, branch: &str) -> anyhow::Result<()> {
    let current = git::run(wt_path, &["rev-parse", "--abbrev-ref", "HEAD"])
        .await?
        .trim()
        .to_string();
    if current == branch {
        return Ok(());
    }
    let switch = git::output(wt_path, &["switch", branch]).await?;
    if switch.status.success() {
        git::cache::flush();
        return Ok(());
    }
    let create = git::output(wt_path, &["switch", "-c", branch]).await?;
    if !create.status.success() {
        return Err(anyhow::anyhow!(
            "worktree at {wt_path} is on branch '{current}' and switching to '{branch}' failed: {}",
            String::from_utf8_lossy(&create.stderr).trim()
        ));
    }
    git::cache::flush();
    Ok(())
}

/// Where a new branch is cut from.
#[derive(Clone, Copy)]
enum BranchPoint<'a> {
    /// The caller's target. Never falls back to another base.
    Exact(&'a str),
    /// What `refresh_main_clone` resolved, then main/master.
    RepoDefault(Option<&'a str>),
}

async fn create_branch(
    branch: &str,
    repo_path: &str,
    point: BranchPoint<'_>,
) -> anyhow::Result<()> {
    let mut candidates: Vec<String> = vec![];
    match point {
        BranchPoint::Exact(base) => candidates.push(format!("origin/{base}")),
        BranchPoint::RepoDefault(base) => {
            if let Some(base) = base {
                candidates.push(format!("origin/{base}"));
            }
            candidates.extend(["origin/main".to_string(), "origin/master".to_string()]);
        }
    }

    for base in &candidates {
        if let Ok(out) = git::output(repo_path, &["branch", branch, base]).await {
            if out.status.success() {
                return Ok(());
            }
        }
    }

    Err(anyhow::anyhow!(
        "Cannot create branch {branch}: none of {} resolve in {repo_path}",
        candidates.join(", ")
    ))
}

/// Delete a stray local branch named `HEAD`; it makes every `HEAD` reference ambiguous in the clone and all its worktrees.
pub(crate) async fn repair_head_branch(repo_or_wt_path: &str) {
    if git::refs::ref_exists(repo_or_wt_path, "refs/heads/HEAD").await {
        tracing::warn!("[git] deleting stray local branch 'HEAD' at {repo_or_wt_path}");
        let _ = git::run(repo_or_wt_path, &["update-ref", "-d", "refs/heads/HEAD"]).await;
    }
}

/// Whether `branch` names this session: a derived task branch carries its short id or tag.
/// An explorer branch is only its title slug, so two same-titled explorers derive one name.
fn names_session(branch: &str, session: &Session, tag: Option<&str>) -> bool {
    let hay = branch.to_lowercase();
    if hay.contains(&session.id.to_lowercase()) {
        return true;
    }
    tag.is_some_and(|t| !t.is_empty() && hay.contains(&t.to_lowercase()))
}

/// Refuse a local branch this session did not derive: its commits belong to other work.
async fn refuse_foreign_branch(
    repo_path: &str,
    repo_label: &str,
    branch: &str,
    branch_is_own: bool,
) -> anyhow::Result<()> {
    if branch_is_own {
        return Ok(());
    }
    let tip = git::run(repo_path, &["log", "-1", "--format=%h %s", branch])
        .await
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let at = if tip.is_empty() {
        String::new()
    } else {
        format!(" at {tip}")
    };
    anyhow::bail!(
        "{repo_label} already has a local branch {branch}{at}, and this session derived that name \
         rather than being given it. To continue that branch on purpose, add the worktree and name \
         the branch. To start fresh, rename this session, or rename or delete that branch."
    )
}

/// Say that a branch of this name already existed and the session continues from it.
/// Provisioning reuses it; a silent reuse looks like a fresh branch.
async fn report_adopted_branch(repo_path: &str, repo_label: &str, branch: &str, session_id: &str) {
    let tip = git::run(repo_path, &["log", "-1", "--format=%h %s", branch])
        .await
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let detail = if tip.is_empty() {
        format!("Delete or rename {branch} if this session should start from the base branch.")
    } else {
        format!("It is at {tip}.\n\nDelete or rename {branch} if this session should start from the base branch.")
    };
    crate::core::events::notice(
        "attention",
        "git",
        format!("{repo_label}: continuing on the existing branch {branch}"),
        Some(detail),
        Some(session_id),
    );
}

/// Fetch the MAIN clone and return its default branch. A fetch failure is reported, not swallowed.
async fn refresh_main_clone(repo_path: &str, repo_label: &str, session_id: &str) -> Option<String> {
    let fetched = git::run(repo_path, &["fetch", "origin"]).await;
    git::cache::flush();
    if let Err(e) = fetched {
        crate::core::events::notice(
            "error",
            "git",
            format!("Could not fetch {repo_label} — its worktree may be based on stale history"),
            Some(e.to_string()),
            Some(session_id),
        );
    }
    repair_head_branch(repo_path).await;

    let default_branch = git::refs::default_branch(repo_path).await?;

    let current = git::run(repo_path, &["rev-parse", "--abbrev-ref", "HEAD"])
        .await
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    let result = if current == default_branch {
        git::run(repo_path, &["pull", "--ff-only"]).await
    } else {
        // Advance the local branch ref without touching MAIN's checkout.
        git::run(
            repo_path,
            &[
                "fetch",
                "origin",
                &format!("{default_branch}:{default_branch}"),
            ],
        )
        .await
    };
    if let Err(e) = result {
        crate::core::events::notice(
            "attention",
            "git",
            format!("{repo_label}: {default_branch} in MAIN could not fast-forward"),
            Some(format!("{e}\n\nNew branches still come from origin/{default_branch}, so this only affects MAIN's own checkout.")),
            Some(session_id),
        );
    }
    Some(default_branch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::db::models::SessionKind;
    use std::path::PathBuf;

    fn session_of(kind: SessionKind, id: &str, title: &str) -> Session {
        Session {
            id: id.to_string(),
            kind,
            title: title.to_string(),
            external_id: None,
            review_project: None,
            review_iid: None,
            created_at: 0,
        }
    }

    /// A derived task branch carries its id, so reopening the task resumes its own work.
    /// An explorer branch is only a title slug: a second explorer of that title must not
    /// inherit the first one's commits.
    #[test]
    fn only_this_sessions_own_branch_is_adopted() {
        let task = session_of(SessionKind::Task, "gh-groove-50", "Harden Groove");
        let derived = naming::default_branch(&task, None);
        assert!(derived.contains("gh-groove-50"), "{derived}");
        assert!(names_session(&derived, &task, None));

        // The same task with a source tag on its branch.
        let tagged = session_of(SessionKind::Task, "notion-abc", "Fix parser");
        let with_tag = naming::default_branch(&tagged, Some("PLAT-42"));
        assert!(names_session(&with_tag, &tagged, Some("PLAT-42")));

        let explorer = session_of(SessionKind::Explorer, "explorer-1234", "front end v2");
        let ex_branch = naming::default_branch(&explorer, None);
        assert_eq!(ex_branch, "explorer/front-end-v2");
        assert!(
            !names_session(&ex_branch, &explorer, None),
            "an explorer branch does not identify its session, so it must be refused"
        );

        // Another session's branch is never this session's.
        assert!(!names_session("fix/other-gh-groove-49", &task, None));
    }

    /// A real clone with a real origin.
    struct Fixture {
        root: PathBuf,
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    async fn git(dir: &str, args: &[&str]) -> String {
        let mut full = vec![
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=T",
            "-c",
            "commit.gpgsign=false",
        ];
        full.extend_from_slice(args);
        git::run(dir, &full)
            .await
            .unwrap_or_else(|e| panic!("git {args:?}: {e}"))
            .trim()
            .to_string()
    }

    impl Fixture {
        /// `release/1.0` sits one commit behind `main`.
        async fn new(name: &str) -> (Self, String) {
            let root = std::env::temp_dir()
                .join(format!("groove-provision-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(&root).unwrap();
            let fixture = Fixture { root: root.clone() };

            let origin = root.join("origin.git");
            std::fs::create_dir_all(&origin).unwrap();
            let origin_s = origin.to_string_lossy().to_string();
            git(&origin_s, &["init", "--bare", "--initial-branch=main", "."]).await;

            let work = root.join("work");
            std::fs::create_dir_all(&work).unwrap();
            let work_s = work.to_string_lossy().to_string();
            git(&work_s, &["init", "--initial-branch=main", "."]).await;
            std::fs::write(work.join("a.txt"), "one\n").unwrap();
            git(&work_s, &["add", "."]).await;
            git(&work_s, &["commit", "-m", "first"]).await;
            git(&work_s, &["remote", "add", "origin", &origin_s]).await;
            git(&work_s, &["push", "origin", "main:release/1.0"]).await;
            std::fs::write(work.join("a.txt"), "two\n").unwrap();
            git(&work_s, &["commit", "-am", "second"]).await;
            git(&work_s, &["push", "origin", "main"]).await;
            git(&work_s, &["fetch", "origin"]).await;
            git(&work_s, &["remote", "set-head", "origin", "main"]).await;

            (fixture, work_s)
        }
    }

    async fn tip(work: &str, git_ref: &str) -> String {
        git(work, &["rev-parse", git_ref]).await
    }

    #[tokio::test]
    async fn a_named_target_is_the_branch_point() {
        let (_fx, work) = Fixture::new("exact").await;
        create_branch("fix/x", &work, BranchPoint::Exact("release/1.0"))
            .await
            .unwrap();
        assert_eq!(
            tip(&work, "fix/x").await,
            tip(&work, "origin/release/1.0").await
        );
        assert_ne!(tip(&work, "fix/x").await, tip(&work, "origin/main").await);
    }

    #[tokio::test]
    async fn a_named_target_that_does_not_resolve_never_falls_back() {
        let (_fx, work) = Fixture::new("nofallback").await;
        let err = create_branch("fix/x", &work, BranchPoint::Exact("no-such-branch"))
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("origin/no-such-branch"), "{err}");
        assert!(!err.contains("origin/main"), "{err}");
    }

    #[tokio::test]
    async fn no_target_branches_from_the_repo_default() {
        let (_fx, work) = Fixture::new("default").await;
        create_branch("fix/x", &work, BranchPoint::RepoDefault(Some("main")))
            .await
            .unwrap();
        assert_eq!(tip(&work, "fix/x").await, tip(&work, "origin/main").await);
    }

    #[test]
    fn a_branch_name_git_would_refuse_never_reaches_the_disk() {
        for good in [
            "fix/parser-42",
            "explorer/try-sqlite-vacuum",
            "release/1.0",
            "feat/a_b-c.d",
            "main",
        ] {
            assert!(validate_branch_name(good).is_ok(), "{good}");
        }

        for bad in [
            "",
            "/etc/cron.d/x",
            "..",
            "../../etc/passwd",
            "fix/../../../etc",
            "fix/x\n",
            "fix/x\u{7f}",
            "fix/two words",
            "-fix/x",
            "fix//x",
            "fix/x/",
            "fix/x~1",
            "fix/x^",
            "fix/x:y",
            "fix/x?",
            "fix/x*",
            "fix/x[0]",
            "fix/x\\y",
            "fix/x.lock",
            "fix/.hidden",
            ".hidden/x",
            "fix/x.",
        ] {
            assert!(validate_branch_name(bad).is_err(), "{bad:?} was accepted");
        }
    }

    #[tokio::test]
    async fn an_unresolved_default_falls_back_to_main() {
        let (_fx, work) = Fixture::new("guess").await;
        create_branch("fix/x", &work, BranchPoint::RepoDefault(None))
            .await
            .unwrap();
        assert_eq!(tip(&work, "fix/x").await, tip(&work, "origin/main").await);
    }
}
