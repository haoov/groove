use sqlx::SqlitePool;
use crate::core::db::store;
use super::commands::load_mr_context;
use super::client::make_client;

/// Separator before the ticket link. Also the marker that finds an existing
/// footer, so re-describing an MR replaces the link instead of stacking copies.
const FOOTER_MARK: &str = "\n\n---\nNotion: ";

/// Put the footer on a description, replacing one that is already there.
///
/// `page_id` is None for synthetic sessions (explorer / review), which have no
/// Notion page — those get the body back with any stray footer removed.
fn apply_footer(description: &str, task_id: &str, page_id: Option<&str>) -> String {
    // An agent told "the link is appended automatically" occasionally writes one
    // anyway, and an update resends the body we produced last time. Either way,
    // cut at the marker so the link cannot stack up.
    let body = match description.find(FOOTER_MARK) {
        Some(at) => &description[..at],
        None => description,
    }
    .trim_end();

    match page_id {
        Some(page) => format!(
            "{body}{FOOTER_MARK}[{task_id}](https://www.notion.so/{})",
            page.replace('-', "")
        ),
        None => body.to_string(),
    }
}

/// Append the task's Notion link to a description.
///
/// The link is derived, never written by the agent — the tool descriptions say so,
/// and both create and update go through here, so an agent that rewrites a whole
/// description cannot drop it.
async fn with_notion_footer(
    description: &str,
    task_id: &str,
    pool: &SqlitePool,
) -> anyhow::Result<String> {
    let page_id = store::sessions::get_opt(pool, task_id)
        .await?
        .and_then(|s| s.notion_page_id);

    Ok(apply_footer(description, task_id, page_id.as_deref()))
}

pub async fn create_mr_impl(payload: serde_json::Value, pool: &SqlitePool) -> anyhow::Result<()> {
    let worktree_id = payload["worktree_id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing worktree_id"))?;
    let title = payload["title"].as_str().unwrap_or("WIP");
    let description = payload["description"].as_str().unwrap_or("");

    let wt = store::worktrees::get(pool, worktree_id).await?;
    let repo = store::repos::get(pool, &wt.repo_id).await?;

    let described = with_notion_footer(description, &wt.session_id, pool).await?;

    let client = make_client(&repo);
    let (remote_id, url) = client
        .create_mr(&repo, &wt.branch, title, &described)
        .await?;

    store::mrs::upsert(pool, worktree_id, client.platform_name(), &remote_id, &url, "open")
        .await?;

    Ok(())
}

pub async fn update_mr_impl(payload: serde_json::Value, pool: &SqlitePool) -> anyhow::Result<()> {
    let mr_id = payload["mr_id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing mr_id"))?;
    let title = payload["title"].as_str();
    let description = payload["description"].as_str();

    let (mr, wt, repo) = load_mr_context(mr_id, pool).await?;

    // A description update REPLACES the whole body, so the footer has to be
    // re-appended here or the ticket link vanishes on every edit.
    let described = match description {
        Some(d) => Some(with_notion_footer(d, &wt.session_id, pool).await?),
        None => None,
    };

    let client = make_client(&repo);
    client
        .update_mr(&repo, &mr.remote_id, title, described.as_deref())
        .await?;

    Ok(())
}

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
    use super::apply_footer;

    const PAGE: &str = "24f1a2b3-c4d5-6789-abcd-ef0123456789";
    const LINK: &str = "Notion: [TASKS2-42](https://www.notion.so/24f1a2b3c4d56789abcdef0123456789)";

    #[test]
    fn appends_the_link() {
        let out = apply_footer("## What\nCuts retention.", "TASKS2-42", Some(PAGE));
        assert_eq!(out, format!("## What\nCuts retention.\n\n---\n{LINK}"));
    }

    #[test]
    fn replaces_an_existing_footer_instead_of_stacking() {
        // What an update resends: the body we produced on create.
        let once = apply_footer("## What\nCuts retention.", "TASKS2-42", Some(PAGE));
        let twice = apply_footer(&once, "TASKS2-42", Some(PAGE));
        assert_eq!(once, twice);
        assert_eq!(twice.matches("Notion: ").count(), 1);
    }

    #[test]
    fn strips_a_footer_the_agent_wrote_itself() {
        let hand_written = "## What\nCuts retention.\n\n---\nNotion: [wrong](https://x)".to_string();
        let out = apply_footer(&hand_written, "TASKS2-42", Some(PAGE));
        assert!(out.ends_with(LINK), "{out}");
        assert!(!out.contains("wrong"), "{out}");
    }

    #[test]
    fn no_page_means_no_footer() {
        let out = apply_footer("## What\nCuts retention.", "explorer-ab12cd", None);
        assert_eq!(out, "## What\nCuts retention.");
    }
}
