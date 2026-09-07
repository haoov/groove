//! Explorer → task conversion: file the task at its provider, then move the
//! worktrees, DB rows and agent session onto the new task id.

use sqlx::SqlitePool;

use crate::core::db::models::{Repo, SessionKind, Worktree};
use crate::core::db::store;
use crate::provider::types::{ProviderId, TaskDraft};

/// The confirmation payload: the explorer to convert and the task to file.
struct ConvertRequest<'a> {
    explorer_id: &'a str,
    provider: ProviderId,
    draft: TaskDraft<'a>,
}

impl<'a> ConvertRequest<'a> {
    fn from_payload(payload: &'a serde_json::Value) -> anyhow::Result<Self> {
        let provider = crate::provider::commands::draft_provider(payload)?;
        Ok(Self {
            explorer_id: payload["explorer_id"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("missing explorer_id"))?,
            provider,
            draft: TaskDraft {
                title: payload["title"].as_str().unwrap_or("Untitled task"),
                body_markdown: payload["body_markdown"].as_str().unwrap_or(""),
                repo: payload["repo"].as_str(),
            },
        })
    }
}

/// Refuse anything that is not a live explorer session.
async fn validate_source(explorer_id: &str, pool: &SqlitePool) -> anyhow::Result<()> {
    match store::sessions::kind_of(pool, explorer_id).await? {
        None => Err(anyhow::anyhow!("no session {explorer_id} to convert")),
        Some(SessionKind::Explorer) => Ok(()),
        Some(kind) => Err(anyhow::anyhow!(
            "{explorer_id} is a {kind:?} session — only explorer sessions convert to tasks"
        )),
    }
}

/// Rename the explorer branch to the task's branch name.
async fn rename_branch(wt: &Worktree, new_branch: &str) -> anyhow::Result<()> {
    // A stray local branch named "HEAD" makes `branch -m` fail as ambiguous.
    crate::worktrees::repair_head_branch(&wt.path).await;

    match crate::core::git::output(&wt.path, &["branch", "-m", new_branch]).await {
        Ok(o) if o.status.success() => {
            crate::core::git::cache::flush();
            Ok(())
        }
        Ok(o) => Err(anyhow::anyhow!(
            "could not rename the branch of {} to {new_branch}: {}",
            wt.path,
            String::from_utf8_lossy(&o.stderr).trim()
        )),
        Err(e) => Err(anyhow::anyhow!("git branch -m failed for {}: {e}", wt.path)),
    }
}

/// `git worktree move`, parents created first.
async fn move_worktree(repo_local_path: &str, from: &str, to: &str) -> anyhow::Result<()> {
    // `git worktree move` does not create missing parent directories.
    if let Some(parent) = std::path::Path::new(to).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match crate::core::git::output(repo_local_path, &["worktree", "move", from, to]).await {
        Ok(o) if o.status.success() => Ok(()),
        Ok(o) => Err(anyhow::anyhow!(
            "could not move {from} to {to}: {}",
            String::from_utf8_lossy(&o.stderr).trim()
        )),
        Err(e) => Err(anyhow::anyhow!("worktree move failed for {from}: {e}")),
    }
}

/// Move the worktree into the task's session dir; returns the new path.
async fn relocate_worktree(
    wt: &Worktree,
    repo: &Repo,
    session_dir: &std::path::Path,
    new_branch: &str,
) -> anyhow::Result<String> {
    let dest = session_dir
        .join(crate::worktrees::naming::worktree_dir(
            &repo.project,
            new_branch,
        ))
        .to_string_lossy()
        .to_string();
    move_worktree(&repo.local_path, &wt.path, &dest).await?;
    Ok(dest)
}

/// One promoted worktree and the disk state to restore if the DB half fails.
struct Promotion {
    worktree_id: String,
    repo_local_path: Option<String>,
    old_branch: String,
    old_path: String,
    new_path: String,
    branch_renamed: bool,
}

/// Rename and relocate every worktree of the explorer. Returns what moved, and a warning per failure.
async fn promote_worktrees(
    explorer_id: &str,
    session_dir: &std::path::Path,
    new_branch: &str,
    pool: &SqlitePool,
) -> anyhow::Result<(Vec<Promotion>, Vec<String>)> {
    let worktrees = store::worktrees::for_session(pool, explorer_id).await?;

    let mut promoted: Vec<Promotion> = vec![];
    let mut warnings: Vec<String> = vec![];

    for wt in &worktrees {
        if let Err(msg) = rename_branch(wt, new_branch).await {
            tracing::error!("[convert] {msg}");
            warnings.push(msg.to_string());
            continue;
        }

        let mut final_path = wt.path.clone();
        let repo = store::repos::get_opt(pool, &wt.repo_id).await?;
        if let Some(repo) = &repo {
            match relocate_worktree(wt, repo, session_dir, new_branch).await {
                Ok(dest) => final_path = dest,
                Err(msg) => {
                    tracing::warn!("[convert] {msg}");
                    warnings.push(msg.to_string());
                }
            }
        }
        promoted.push(Promotion {
            worktree_id: wt.id.clone(),
            repo_local_path: repo.map(|r| r.local_path),
            old_branch: wt.branch.clone(),
            old_path: wt.path.clone(),
            new_path: final_path,
            branch_renamed: true,
        });
    }

    Ok((promoted, warnings))
}

/// Put every promoted worktree back where the DB rows still point.
async fn rollback_promotions(promoted: &[Promotion]) {
    for p in promoted {
        let mut path = p.new_path.clone();
        if p.new_path != p.old_path {
            match p.repo_local_path.as_deref() {
                Some(local) => match move_worktree(local, &p.new_path, &p.old_path).await {
                    Ok(()) => {
                        tracing::warn!(
                            "[convert] rolled back the move of {path} to {}",
                            p.old_path
                        );
                        path = p.old_path.clone();
                    }
                    Err(msg) => tracing::error!("[convert] rollback failed: {msg}"),
                },
                None => tracing::error!(
                    "[convert] cannot roll back the move of {path}: its repo is gone"
                ),
            }
        }
        if p.branch_renamed {
            match crate::core::git::output(&path, &["branch", "-m", &p.old_branch]).await {
                Ok(o) if o.status.success() => tracing::warn!(
                    "[convert] rolled back the branch of {path} to {}",
                    p.old_branch
                ),
                Ok(o) => tracing::error!(
                    "[convert] rollback failed: could not rename the branch of {path} back to {}: {}",
                    p.old_branch,
                    String::from_utf8_lossy(&o.stderr).trim()
                ),
                Err(e) => tracing::error!("[convert] rollback failed: git branch -m in {path}: {e}"),
            }
        }
    }
    crate::core::git::cache::flush();
}

/// Write the explorer's agent session id into the task's `.agent_session_id` file.
fn handoff_agent_session(explorer_id: &str, session_dir: &std::path::Path) {
    let session_uuid = crate::agent_manager::task_session_uuid(explorer_id);
    let sid_path = session_dir.join(".agent_session_id");
    let _ = std::fs::create_dir_all(session_dir);
    if let Err(e) = std::fs::write(&sid_path, session_uuid) {
        tracing::warn!(
            "[convert] could not persist agent session handoff at {}: {e}",
            sid_path.display()
        );
    }
}

/// Remove the empty explorer dir. Keep `remove_dir`: a worktree that failed to move stays inside.
fn cleanup_explorer_dir(explorer_dir: &std::path::Path) {
    let _ = std::fs::remove_file(explorer_dir.join(".agent_session_id"));
    if let Err(e) = std::fs::remove_dir(explorer_dir) {
        if explorer_dir.exists() {
            tracing::warn!(
                "[convert] could not remove explorer dir {}: {e}",
                explorer_dir.display()
            );
        }
    }
}

/// The session shape the branch namer needs, before the row is re-keyed.
fn adopted_session(short_id: &str, title: &str) -> crate::core::db::models::Session {
    crate::core::db::models::Session {
        id: short_id.to_string(),
        kind: crate::core::db::models::SessionKind::Task,
        title: title.to_string(),
        external_id: Some(String::new()),
        review_project: None,
        review_iid: None,
        created_at: 0,
    }
}

/// Convert an explorer session into a task; bridge op `task.create_from_explorer`.
pub async fn create_task_from_explorer_impl(
    payload: serde_json::Value,
    pool: &SqlitePool,
) -> anyhow::Result<serde_json::Value> {
    let req = ConvertRequest::from_payload(&payload)?;
    validate_source(req.explorer_id, pool).await?;

    let provider = crate::provider::get(req.provider)?;
    let filed = provider.create_task(&req.draft).await?;
    let short_id =
        crate::provider::commands::mint_short_id(pool, provider, &filed, &mut Default::default())
            .await?;

    let now = chrono::Utc::now().timestamp();
    let new_branch = crate::worktrees::naming::default_branch(
        &adopted_session(&short_id, req.draft.title),
        filed.branch_tag.as_deref(),
    );
    let session_dir = crate::worktrees::session_dir(&short_id);

    let (promoted, branch_warnings) =
        promote_worktrees(req.explorer_id, &session_dir, &new_branch, pool).await?;
    let switched: Vec<(String, String)> = promoted
        .iter()
        .map(|p| (p.worktree_id.clone(), p.new_path.clone()))
        .collect();

    let task = crate::provider::mirror_row(&short_id, &filed);
    if let Err(e) =
        store::sessions::adopt_explorer(pool, req.explorer_id, &task, &switched, &new_branch).await
    {
        rollback_promotions(&promoted).await;
        return Err(e.into());
    }

    handoff_agent_session(req.explorer_id, &session_dir);
    cleanup_explorer_dir(&crate::worktrees::session_dir(req.explorer_id));

    let mut out = crate::provider::commands::filed_response(&short_id, &filed, now);
    out["branch_warnings"] = serde_json::json!(branch_warnings);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::{promote_worktrees, rollback_promotions};
    use crate::core::db::models::Repo;
    use crate::core::db::{store, test_pool};
    use std::path::PathBuf;

    struct Tmp(PathBuf);
    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
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
        crate::core::git::run(dir, &full)
            .await
            .unwrap_or_else(|e| panic!("git {args:?}: {e}"))
            .trim()
            .to_string()
    }

    #[tokio::test]
    async fn a_failed_adoption_puts_the_worktrees_back() {
        let root = std::env::temp_dir().join(format!("groove-convert-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let tmp = Tmp(root.clone());

        let clone = root.join("main/groove");
        std::fs::create_dir_all(&clone).unwrap();
        let clone_s = clone.to_string_lossy().to_string();
        git(&clone_s, &["init", "--initial-branch=main", "."]).await;
        std::fs::write(clone.join("a.txt"), "one\n").unwrap();
        git(&clone_s, &["add", "."]).await;
        git(&clone_s, &["commit", "-m", "first"]).await;

        let explorer_dir = root.join("worktrees/explorer-1");
        let wt_path = explorer_dir.join("groove/explorer/try-it");
        std::fs::create_dir_all(wt_path.parent().unwrap()).unwrap();
        git(
            &clone_s,
            &[
                "worktree",
                "add",
                "-b",
                "explorer/try-it",
                &wt_path.to_string_lossy(),
                "main",
            ],
        )
        .await;

        let pool = test_pool().await;
        store::repos::upsert(
            &pool,
            &Repo {
                id: "g/groove".into(),
                host: "github.com".into(),
                group_path: "g".into(),
                project: "groove".into(),
                local_path: clone_s.clone(),
            },
        )
        .await
        .unwrap();
        store::sessions::create_explorer(&pool, "explorer-1", "Try it")
            .await
            .unwrap();
        store::repos::attach(&pool, "explorer-1", "g/groove")
            .await
            .unwrap();
        store::worktrees::upsert(
            &pool,
            "explorer-1",
            "g/groove",
            "explorer/try-it",
            &wt_path.to_string_lossy(),
        )
        .await
        .unwrap();

        let task_dir = root.join("worktrees/gh-groove-7");
        let (promoted, warnings) =
            promote_worktrees("explorer-1", &task_dir, "feat/try-it-7", &pool)
                .await
                .unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");
        let moved = task_dir.join("groove/feat/try-it-7");
        assert!(moved.is_dir(), "the promotion did not move the worktree");

        rollback_promotions(&promoted).await;

        assert!(wt_path.is_dir(), "the worktree stayed at the task path");
        assert!(!moved.exists(), "the task path was left behind");
        assert_eq!(
            git(
                &wt_path.to_string_lossy(),
                &["rev-parse", "--abbrev-ref", "HEAD"]
            )
            .await,
            "explorer/try-it",
            "the branch was left renamed"
        );
        drop(tmp);
    }
}
