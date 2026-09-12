use super::client::make_client;
use super::commands::load_mr_context;
use crate::core::db::models::{Repo, Worktree};
use crate::core::db::store;
use crate::core::git;
use sqlx::SqlitePool;

/// Separator before the task link; also the marker that finds an existing footer.
const FOOTER_MARK: &str = "\n\n---\nTask: ";
/// The Notion-era footer marker, still cut.
const LEGACY_FOOTER_MARK: &str = "\n\n---\nNotion: ";

/// Put the footer on a description, replacing one already there; a `None` url strips it.
fn apply_footer(description: &str, task_id: &str, url: Option<&str>) -> String {
    // Cut at the earliest marker so the link cannot stack.
    let cut = [FOOTER_MARK, LEGACY_FOOTER_MARK]
        .iter()
        .filter_map(|m| description.find(m))
        .min();
    let body = match cut {
        Some(at) => &description[..at],
        None => description,
    }
    .trim_end();

    match url {
        Some(url) => format!("{body}{FOOTER_MARK}[{task_id}]({url})"),
        None => body.to_string(),
    }
}

/// Append the task's link to a description.
async fn with_task_footer(
    description: &str,
    task_id: &str,
    pool: &SqlitePool,
) -> anyhow::Result<String> {
    let url = match crate::provider::resolve(pool, task_id).await {
        Ok((provider, key)) => Some(provider.task_url(&key)),
        // Explorer and review sessions have no task behind them.
        Err(_) => None,
    };
    Ok(apply_footer(description, task_id, url.as_deref()))
}

/// The branch an MR from this worktree would land on.
pub async fn mr_target_for(pool: &SqlitePool, worktree_id: &str) -> anyhow::Result<String> {
    let wt = store::worktrees::get(pool, worktree_id).await?;
    let repo = store::repos::get(pool, &wt.repo_id).await?;
    Ok(target_for(&repo, &wt).await)
}

/// The worktree's base, else the repo default.
async fn target_for(repo: &Repo, wt: &Worktree) -> String {
    if let Some(base) = wt
        .base_ref
        .as_deref()
        .map(str::trim)
        .filter(|b| !b.is_empty())
    {
        return base.to_string();
    }
    git::refs::default_branch(&repo.local_path)
        .await
        .unwrap_or_else(|| "main".to_string())
}

#[tracing::instrument(skip_all, fields(worktree = payload["worktree_id"].as_str()))]
pub async fn create_mr_impl(payload: serde_json::Value, pool: &SqlitePool) -> anyhow::Result<()> {
    let worktree_id = payload["worktree_id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing worktree_id"))?;
    let title = payload["title"].as_str().unwrap_or("WIP");
    let description = payload["description"].as_str().unwrap_or("");

    let wt = store::worktrees::get(pool, worktree_id).await?;
    let repo = store::repos::get(pool, &wt.repo_id).await?;

    let target = target_for(&repo, &wt).await;
    let described = with_task_footer(description, &wt.session_id, pool).await?;

    let client = make_client(&repo);
    let (remote_id, url) = client
        .create_mr(&repo, &wt.branch, &target, title, &described)
        .await?;

    store::mrs::upsert(
        pool,
        worktree_id,
        client.platform_name(),
        &remote_id,
        &url,
        "open",
    )
    .await?;

    Ok(())
}

#[tracing::instrument(skip_all, fields(mr = payload["mr_id"].as_str()))]
pub async fn update_mr_impl(payload: serde_json::Value, pool: &SqlitePool) -> anyhow::Result<()> {
    let mr_id = payload["mr_id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing mr_id"))?;
    let title = payload["title"].as_str();
    let description = payload["description"].as_str();

    let (mr, wt, repo) = load_mr_context(mr_id, pool).await?;

    // A description update replaces the whole body; re-append the footer.
    let described = match description {
        Some(d) => Some(with_task_footer(d, &wt.session_id, pool).await?),
        None => None,
    };

    let client = make_client(&repo);
    client
        .update_mr(&repo, &mr.remote_id, title, described.as_deref())
        .await?;

    Ok(())
}

#[tracing::instrument(skip_all, fields(mr = payload["mr_id"].as_str()))]
pub async fn close_mr_impl(payload: serde_json::Value, pool: &SqlitePool) -> anyhow::Result<()> {
    let mr_id = payload["mr_id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing mr_id"))?;

    let (mr, _wt, repo) = load_mr_context(mr_id, pool).await?;

    let client = make_client(&repo);
    client.close_mr(&repo, &mr.remote_id).await?;

    store::mrs::set_state(pool, mr_id, "closed").await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{apply_footer, target_for};
    use crate::core::db::models::{Repo, Worktree};

    const PAGE: &str = "https://www.notion.so/24f1a2b3c4d56789abcdef0123456789";
    const LINK: &str = "Task: [TASKS2-42](https://www.notion.so/24f1a2b3c4d56789abcdef0123456789)";

    #[test]
    fn appends_the_link() {
        let out = apply_footer("## What\nCuts retention.", "TASKS2-42", Some(PAGE));
        assert_eq!(out, format!("## What\nCuts retention.\n\n---\n{LINK}"));
    }

    #[test]
    fn replaces_an_existing_footer_instead_of_stacking() {
        let once = apply_footer("## What\nCuts retention.", "TASKS2-42", Some(PAGE));
        let twice = apply_footer(&once, "TASKS2-42", Some(PAGE));
        assert_eq!(once, twice);
        assert_eq!(twice.matches("Task: ").count(), 1);
    }

    #[test]
    fn strips_a_footer_the_agent_wrote_itself() {
        let hand_written = "## What\nCuts retention.\n\n---\nTask: [wrong](https://x)".to_string();
        let out = apply_footer(&hand_written, "TASKS2-42", Some(PAGE));
        assert!(out.ends_with(LINK), "{out}");
        assert!(!out.contains("wrong"), "{out}");
    }

    #[test]
    fn replaces_the_pre_provider_footer() {
        let old = "## What\nCuts retention.\n\n---\nNotion: [TASKS2-42](https://www.notion.so/x)";
        let out = apply_footer(old, "TASKS2-42", Some(PAGE));
        assert!(out.ends_with(LINK), "{out}");
        assert!(!out.contains("Notion: "), "{out}");
    }

    fn repo() -> Repo {
        Repo {
            id: "github.com/haoov/groove".into(),
            host: "github.com".into(),
            group_path: "haoov".into(),
            project: "groove".into(),
            local_path: "/nowhere".into(),
        }
    }

    fn worktree(base_ref: Option<&str>) -> Worktree {
        Worktree {
            id: "wt-1".into(),
            session_id: "gh-haoov-groove-22".into(),
            repo_id: "github.com/haoov/groove".into(),
            branch: "fix/x-22".into(),
            path: "/wt/fix/x-22".into(),
            base_ref: base_ref.map(str::to_string),
            created_at: 0,
        }
    }

    #[tokio::test]
    async fn the_mr_targets_the_branch_the_worktree_was_based_on() {
        assert_eq!(
            target_for(&repo(), &worktree(Some("release/1.0"))).await,
            "release/1.0"
        );
    }

    #[tokio::test]
    async fn a_blank_base_is_not_a_target() {
        assert_eq!(target_for(&repo(), &worktree(Some("  "))).await, "main");
    }

    #[test]
    fn no_page_means_no_footer() {
        let out = apply_footer("## What\nCuts retention.", "explorer-ab12cd", None);
        assert_eq!(out, "## What\nCuts retention.");
    }
}
