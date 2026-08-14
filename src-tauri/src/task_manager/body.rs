//! Replacing a task's page body from markdown.
//!
//! Notion pages are blocks, not text, and our markdown bridge only covers the
//! block types a task body normally uses. Replacing children therefore DESTROYS
//! anything outside that set — embeds, images, child databases, toggles — and
//! gives every surviving block a new id, which detaches block-level comments.
//!
//! So this refuses by default when the page holds something it cannot faithfully
//! rebuild, and names what it found. `force` exists for when you have looked at
//! the list and accept the loss.

use sqlx::SqlitePool;

use super::notion::{markdown_to_blocks, notion_patch};

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
    let token = payload["token"].as_str().unwrap_or_default();
    let page_id = payload["notion_page_id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing notion_page_id"))?;
    let markdown = payload["markdown"].as_str().unwrap_or_default();
    let force = payload["force"].as_bool().unwrap_or(false);

    let existing = super::notion::fetch_block_children(page_id, token).await?;

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
        notion_patch(
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
        match notion_patch(
            token,
            &format!("v1/blocks/{id}"),
            &serde_json::json!({ "archived": true }),
        )
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
    bridge: tauri::State<'_, crate::confirmation_bridge::Bridge>,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<String, String> {
    bridge
        .post(
            &pool,
            crate::ops::NOTION_BODY,
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
