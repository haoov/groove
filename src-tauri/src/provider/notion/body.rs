//! Reading and replacing a task page's body. A replace destroys blocks that
//! markdown cannot rebuild and detaches block comments; it refuses unless `force`.

use super::markdown::{blocks_to_markdown, markdown_to_blocks};

const MAX_BLOCK_PAGES: usize = 30;

/// Fetch one block's direct children, paginating past Notion's 100-block cap.
async fn fetch_block_children(
    block_id: &str,
    token: &str,
) -> anyhow::Result<Vec<serde_json::Value>> {
    super::api::paginate_get(
        token,
        &format!("v1/blocks/{block_id}/children"),
        MAX_BLOCK_PAGES,
    )
    .await
}

/// Fetch the page blocks. Table rows are attached to their table as `__children`.
#[tracing::instrument(skip_all, fields(page = notion_page_id))]
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
    for ((i, id), rows) in table_ids
        .iter()
        .zip(futures_util::future::join_all(fetches).await)
    {
        match rows {
            Ok(rows) => blocks[*i]["__children"] = serde_json::Value::Array(rows),
            Err(e) => tracing::warn!("table rows fetch failed for {id}: {e}"),
        }
    }
    Ok(blocks)
}

/// Fetch the task template as markdown. Checks first that the id is a readable page.
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

/// Block types that survive markdown -> Notion -> markdown unchanged. `table` is
/// absent: the reader renders one, `markdown_to_blocks` cannot build one back.
const ROUND_TRIPPABLE: [&str; 10] = [
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
];

/// Notion accepts at most 100 children per append.
const APPEND_BATCH: usize = 100;

/// What a markdown rewrite of the page would destroy, deduplicated: block types
/// markdown cannot represent, and nested children, which the reader never fetched.
fn lossy_types(blocks: &[serde_json::Value]) -> Vec<String> {
    let mut found: Vec<String> = vec![];
    let mut push = |what: String| {
        if !found.contains(&what) {
            found.push(what);
        }
    };
    for b in blocks {
        let Some(kind) = b["type"].as_str() else {
            continue;
        };
        if !ROUND_TRIPPABLE.contains(&kind) {
            push(kind.to_string());
            continue;
        }
        if b["has_children"].as_bool() == Some(true) {
            push(format!("{kind} with nested blocks"));
        }
    }
    found
}

/// Replace the page body from markdown. Refuses when the page holds blocks
/// markdown cannot rebuild, unless forced.
pub(crate) async fn replace(
    token: &str,
    page_id: &str,
    markdown: &str,
    force: bool,
) -> anyhow::Result<crate::provider::types::BodyWrite> {
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

    // Append before archiving. Do not reorder: a failed append must leave the page intact.
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
        let Some(id) = block["id"].as_str() else {
            continue;
        };
        match super::api::patch(
            token,
            &format!("v1/blocks/{id}"),
            &serde_json::json!({ "archived": true }),
        )
        .await
        {
            Ok(_) => removed += 1,
            Err(e) => tracing::warn!("[task body] could not archive block {id}: {e}"),
        }
    }

    let _ = removed;
    Ok(crate::provider::types::BodyWrite {
        blocks_written: appended,
    })
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

    #[test]
    fn a_callout_counts_as_loss() {
        let blocks = vec![serde_json::json!({ "type": "callout", "id": "1" })];
        assert_eq!(lossy_types(&blocks), vec!["callout"]);
    }

    #[test]
    fn a_table_counts_as_loss() {
        let blocks = vec![serde_json::json!({ "type": "table", "id": "1" })];
        assert_eq!(lossy_types(&blocks), vec!["table"]);
    }

    #[test]
    fn nested_children_count_as_loss() {
        let blocks = vec![
            serde_json::json!({ "type": "bulleted_list_item", "id": "1", "has_children": true }),
            serde_json::json!({ "type": "paragraph", "id": "2", "has_children": false }),
        ];
        assert_eq!(
            lossy_types(&blocks),
            vec!["bulleted_list_item with nested blocks"]
        );
    }
}
