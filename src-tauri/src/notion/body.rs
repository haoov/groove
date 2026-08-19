//! Reading and replacing a task page's body.
//!
//! Notion pages are blocks, not text, and the markdown bridge only covers the
//! block types a task body normally uses. Replacing children therefore DESTROYS
//! anything outside that set — embeds, images, child databases, toggles — and
//! gives every surviving block a new id, which detaches block-level comments.
//! So the replace refuses by default when the page holds something it cannot
//! faithfully rebuild, and names what it found; `force` accepts the loss.

use sqlx::SqlitePool;

use crate::core::config;

use super::markdown::{blocks_to_markdown, markdown_to_blocks};

/// Blocks pagination backstop: 30 pages × 100 blocks is far above any task body.
const MAX_BLOCK_PAGES: usize = 30;

/// Fetch one block's direct children, paginating past Notion's 100-block cap.
async fn fetch_block_children(
    block_id: &str,
    token: &str,
) -> anyhow::Result<Vec<serde_json::Value>> {
    super::api::paginate_get(token, &format!("v1/blocks/{block_id}/children"), MAX_BLOCK_PAGES).await
}

/// Fetch the Notion page blocks — shared impl callable from MCP and Tauri commands.
/// Table blocks get their rows (children) attached as `__children` so the UI and
/// the markdown renderer can show them (rows are NOT part of the parent payload).
/// Row fetches run concurrently — a page with several tables pays one round trip.
pub async fn get_task_body_impl(
    notion_page_id: &str,
    token: &str,
) -> anyhow::Result<Vec<serde_json::Value>> {
    let mut blocks = fetch_block_children(notion_page_id, token).await?;

    let table_ids: Vec<(usize, String)> = blocks
        .iter()
        .enumerate()
        .filter(|(_, b)| {
            b["type"].as_str() == Some("table") && b["has_children"].as_bool() == Some(true)
        })
        .filter_map(|(i, b)| b["id"].as_str().map(|id| (i, id.to_string())))
        .collect();

    let fetches = table_ids
        .iter()
        .map(|(_, id)| fetch_block_children(id, token));
    for ((i, id), rows) in table_ids.iter().zip(futures_util::future::join_all(fetches).await) {
        match rows {
            Ok(rows) => blocks[*i]["__children"] = serde_json::Value::Array(rows),
            Err(e) => tracing::warn!("table rows fetch failed for {id}: {e}"),
        }
    }
    Ok(blocks)
}

/// Fetch the task template as markdown, validating the configured id first —
/// a database id (or an inaccessible/archived page) must fail with a clear,
/// actionable error instead of handing junk blocks to the agent.
pub async fn template_markdown(page_id: &str, token: &str) -> anyhow::Result<String> {
    if let Err(e) = super::api::get(token, &format!("v1/pages/{page_id}")).await {
        let msg = e.to_string();
        if msg.contains("is a database") {
            return Err(anyhow::anyhow!(
                "notion.task_template_page_id ({page_id}) is a DATABASE id, not a page id. \
                 Point it at the template PAGE (open the template in Notion → Copy link → use that page's id)."
            ));
        }
        return Err(anyhow::anyhow!(
            "notion.task_template_page_id ({page_id}) is not a readable page \
             (is it shared with the integration?): {msg}"
        ));
    }
    let blocks = get_task_body_impl(page_id, token).await?;
    if blocks.is_empty() {
        return Err(anyhow::anyhow!(
            "the task template page ({page_id}) has no content — add the template body to that page"
        ));
    }
    Ok(blocks_to_markdown(&blocks))
}

// ─── Replace ──────────────────────────────────────────────────────────────────

/// Block types that survive markdown → Notion → markdown unchanged. Anything
/// else either can't be produced from markdown or would come back as a plain
/// paragraph.
const ROUND_TRIPPABLE: [&str; 11] = [
    "paragraph",
    "heading_1",
    "heading_2",
    "heading_3",
    "bulleted_list_item",
    "numbered_list_item",
    "code",
    "quote",
    "divider",
    "to_do",
    "table",
];

/// Notion accepts at most 100 children per append.
const APPEND_BATCH: usize = 100;

/// Block types on the page that markdown cannot represent, deduplicated.
fn lossy_types(blocks: &[serde_json::Value]) -> Vec<String> {
    let mut found: Vec<String> = vec![];
    for b in blocks {
        let Some(kind) = b["type"].as_str() else { continue };
        if ROUND_TRIPPABLE.contains(&kind) || found.iter().any(|f| f == kind) {
            continue;
        }
        found.push(kind.to_string());
    }
    found
}

/// Confirmation-bridge path for `notion.body`.
///
/// Gated even when it comes from the UI: it is the one Notion write that can
/// destroy content the app never showed you.
pub async fn update_body_impl(
    payload: serde_json::Value,
    _pool: &SqlitePool,
) -> anyhow::Result<serde_json::Value> {
    let cfg = config::require()?;
    let token = &cfg.notion.token;
    let page_id = payload["notion_page_id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing notion_page_id"))?;
    let markdown = payload["markdown"].as_str().unwrap_or_default();
    let force = payload["force"].as_bool().unwrap_or(false);

    let existing = fetch_block_children(page_id, token).await?;

    let lossy = lossy_types(&existing);
    if !lossy.is_empty() && !force {
        return Err(anyhow::anyhow!(
            "this page contains blocks markdown can't rebuild ({}) — saving would delete them. \
             Edit the body in Notion instead, or re-save with force to accept the loss.",
            lossy.join(", ")
        ));
    }

    let new_blocks = markdown_to_blocks(markdown);
    if new_blocks.is_empty() && !markdown.trim().is_empty() {
        return Err(anyhow::anyhow!(
            "the markdown produced no Notion blocks — refusing to empty the page"
        ));
    }

    // Append first, then archive the old blocks: if the append fails the page is
    // left intact (duplicated content is recoverable, an emptied page is not).
    let mut appended = 0usize;
    for chunk in new_blocks.chunks(APPEND_BATCH) {
        super::api::patch(
            token,
            &format!("v1/blocks/{page_id}/children"),
            &serde_json::json!({ "children": chunk }),
        )
        .await?;
        appended += chunk.len();
    }

    let mut removed = 0usize;
    for block in &existing {
        let Some(id) = block["id"].as_str() else { continue };
        // Notion has no bulk delete; archiving each is the documented way.
        match super::api::patch(token, &format!("v1/blocks/{id}"), &serde_json::json!({ "archived": true }))
            .await
        {
            Ok(_) => removed += 1,
            // A block that won't archive leaves stale content behind, which is
            // visible and fixable — worth reporting, not worth failing over.
            Err(e) => tracing::warn!("[task body] could not archive block {id}: {e}"),
        }
    }

    Ok(serde_json::json!({
        "ok": true,
        "blocks_written": appended,
        "blocks_replaced": removed,
        "message": format!("Task body updated ({appended} blocks)"),
    }))
}

// ─── IPC ──────────────────────────────────────────────────────────────────────

/// The ticket body as markdown — what the overview renders. Going through
/// markdown (rather than a bespoke Notion-block renderer) means one renderer for
/// task and MR descriptions, Notion's inline annotations survive, AND markdown
/// typed literally into Notion (`**bold**`, backticks) displays as intended.
#[tauri::command]
pub async fn get_task_body_markdown(notion_page_id: String) -> Result<String, String> {
    let cfg = config::require().map_err(|e| e.to_string())?;
    let blocks = get_task_body_impl(&notion_page_id, &cfg.notion.token)
        .await
        .map_err(|e| e.to_string())?;
    Ok(blocks_to_markdown(&blocks))
}

/// Queue a body replacement from the UI.
///
/// Goes through the confirmation bridge even though a human asked for it: this is
/// the one write that can delete blocks the app never displayed, so it gets a
/// preview and an explicit yes like an agent's write would.
#[tauri::command]
pub async fn request_task_body_update(
    notion_page_id: String,
    task_id: String,
    markdown: String,
    force: bool,
    bridge: tauri::State<'_, crate::approvals::Bridge>,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<String, String> {
    bridge
        .post(
            &pool,
            crate::approvals::ops::NOTION_BODY,
            serde_json::json!({
                "notion_page_id": notion_page_id,
                "task_id": task_id,
                "markdown": markdown,
                "force": force,
            }),
            "ui",
            Some(&task_id),
        )
        .await
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_page_of_ordinary_blocks_is_safe() {
        let blocks = vec![
            serde_json::json!({ "type": "paragraph", "id": "1" }),
            serde_json::json!({ "type": "heading_2", "id": "2" }),
            serde_json::json!({ "type": "to_do", "id": "3" }),
        ];
        assert!(lossy_types(&blocks).is_empty());
    }

    #[test]
    fn unrepresentable_blocks_are_named_once_each() {
        let blocks = vec![
            serde_json::json!({ "type": "paragraph", "id": "1" }),
            serde_json::json!({ "type": "image", "id": "2" }),
            serde_json::json!({ "type": "child_database", "id": "3" }),
            serde_json::json!({ "type": "image", "id": "4" }),
        ];
        assert_eq!(lossy_types(&blocks), vec!["image", "child_database"]);
    }

    /// A callout reads as markdown but comes back as a paragraph, so it counts as
    /// loss even though the text survives.
    #[test]
    fn a_callout_counts_as_loss() {
        let blocks = vec![serde_json::json!({ "type": "callout", "id": "1" })];
        assert_eq!(lossy_types(&blocks), vec!["callout"]);
    }
}
