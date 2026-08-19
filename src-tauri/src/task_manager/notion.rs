use crate::core::db::models::NotionTask;
use crate::core::timing::timed;
use crate::core::config::NotionConfig;

pub(super) const NOTION_BASE: &str = "https://api.notion.com";
pub(super) const NOTION_VERSION: &str = "2022-06-28";
pub(super) fn notion_client(token: &str) -> anyhow::Result<reqwest::Client> {
    use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {token}"))
            .map_err(|e| anyhow::anyhow!("invalid token: {e}"))?,
    );
    headers.insert(
        "Notion-Version",
        HeaderValue::from_static(NOTION_VERSION),
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    Ok(reqwest::Client::builder().default_headers(headers).build()?)
}

pub(super) async fn notion_get(token: &str, path: &str) -> anyhow::Result<serde_json::Value> {
    let url = format!("{NOTION_BASE}/{path}");
    timed("http", format!("notion GET {path}"), async {
        let resp = notion_client(token)?.get(&url).send().await?;
        let status = resp.status();
        let body: serde_json::Value = resp.json().await?;
        if !status.is_success() {
            return Err(anyhow::anyhow!("Notion GET {path} failed {status}: {body}"));
        }
        Ok(body)
    })
    .await
}

pub(super) async fn notion_post(
    token: &str,
    path: &str,
    body: &serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    let url = format!("{NOTION_BASE}/{path}");
    timed("http", format!("notion POST {path}"), async {
        let resp = notion_client(token)?.post(&url).json(body).send().await?;
        let status = resp.status();
        let out: serde_json::Value = resp.json().await?;
        if !status.is_success() {
            return Err(anyhow::anyhow!("Notion POST {path} failed {status}: {out}"));
        }
        Ok(out)
    })
    .await
}

pub(super) async fn notion_patch(
    token: &str,
    path: &str,
    body: &serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    let url = format!("{NOTION_BASE}/{path}");
    timed("http", format!("notion PATCH {path}"), async {
        let resp = notion_client(token)?.patch(&url).json(body).send().await?;
        let status = resp.status();
        let out: serde_json::Value = resp.json().await?;
        if !status.is_success() {
            return Err(anyhow::anyhow!("Notion PATCH {path} failed {status}: {out}"));
        }
        Ok(out)
    })
    .await
}

fn extract_title(props: &serde_json::Value) -> Option<String> {
    for key in &["Name", "Title", "title"] {
        if let Some(prop) = props.get(key) {
            if let Some(title_arr) = prop["title"].as_array() {
                let text: String = title_arr
                    .iter()
                    .filter_map(|t| t["plain_text"].as_str())
                    .collect();
                if !text.is_empty() {
                    return Some(text);
                }
            }
        }
    }
    if let Some(obj) = props.as_object() {
        for (_, v) in obj {
            if v["type"].as_str() == Some("title") {
                if let Some(arr) = v["title"].as_array() {
                    let text: String = arr
                        .iter()
                        .filter_map(|t| t["plain_text"].as_str())
                        .collect();
                    if !text.is_empty() {
                        return Some(text);
                    }
                }
            }
        }
    }
    None
}

fn extract_status(props: &serde_json::Value, prop_name: &str) -> Option<String> {
    let prop = props.get(prop_name)?;
    if let Some(name) = prop["status"]["name"].as_str() {
        return Some(name.to_string());
    }
    if let Some(name) = prop["select"]["name"].as_str() {
        return Some(name.to_string());
    }
    None
}

fn extract_select(props: &serde_json::Value, prop_name: &str) -> Option<String> {
    props.get(prop_name)?["select"]["name"]
        .as_str()
        .map(|s| s.to_string())
}

fn extract_unique_id(props: &serde_json::Value) -> Option<String> {
    let obj = props.as_object()?;
    for (_key, val) in obj {
        if val["type"].as_str() == Some("unique_id") {
            let num = val["unique_id"]["number"].as_u64()?;
            let prefix = val["unique_id"]["prefix"].as_str().unwrap_or("");
            return if prefix.is_empty() {
                Some(format!("{num}"))
            } else {
                Some(format!("{prefix}-{num}"))
            };
        }
    }
    None
}

pub(super) fn page_to_task(page: &serde_json::Value, cfg: &NotionConfig) -> anyhow::Result<NotionTask> {
    let page_id = page["id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("page missing id"))?
        .to_string();

    let props = &page["properties"];

    let short_id = extract_unique_id(props)
        .ok_or_else(|| anyhow::anyhow!("page {page_id} has no unique_id"))?;

    let title = extract_title(props).unwrap_or_else(|| format!("(untitled) {short_id}"));

    let status = extract_status(props, &cfg.properties.status)
        .unwrap_or_else(|| cfg.status_map.in_progress.clone());

    let priority = cfg
        .properties
        .priority
        .as_deref()
        .and_then(|k| extract_select(props, k));

    Ok(NotionTask {
        page_id,
        short_id,
        title,
        status,
        priority,
        synced_at: chrono::Utc::now().timestamp(),
    })
}

pub(super) fn parse_short_id_number(short_id: &str) -> Option<u64> {
    // "PLAT-42" → 42,  "42" → 42
    short_id.rsplit('-').next().and_then(|s| s.parse().ok())
}

/// Query the Sprint DB and return page IDs of sprints with "Sprint status" = "Current".
pub(super) async fn current_sprint_ids(token: &str, sprint_db_id: &str) -> Vec<String> {
    let body = serde_json::json!({
        "filter": {
            "property": "Sprint status",
            "status": { "equals": "Current" }
        }
    });
    match notion_post(token, &format!("v1/databases/{sprint_db_id}/query"), &body).await {
        Ok(resp) => resp["results"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|p| p["id"].as_str().map(|s| s.to_string()))
            .collect(),
        Err(e) => {
            tracing::warn!("[sprint filter] sprint DB query failed — check integration permissions: {e}");
            vec![]
        }
    }
}

/// Fetch one block's direct children, paginating past Notion's 100-block cap.
pub(super) async fn fetch_block_children(
    block_id: &str,
    token: &str,
) -> anyhow::Result<Vec<serde_json::Value>> {
    let mut blocks: Vec<serde_json::Value> = vec![];
    let mut cursor: Option<String> = None;
    loop {
        let path = match &cursor {
            Some(c) => format!("v1/blocks/{block_id}/children?page_size=100&start_cursor={c}"),
            None => format!("v1/blocks/{block_id}/children?page_size=100"),
        };
        let resp = notion_get(token, &path).await?;
        if let Some(arr) = resp["results"].as_array() {
            blocks.extend(arr.iter().cloned());
        }
        if resp["has_more"].as_bool() == Some(true) {
            match resp["next_cursor"].as_str() {
                Some(c) => cursor = Some(c.to_string()),
                None => break,
            }
        } else {
            break;
        }
    }
    Ok(blocks)
}

/// Fetch the Notion page blocks — shared impl callable from MCP and Tauri commands.
/// Table blocks get their rows (children) attached as `__children` so the UI and
/// the markdown renderer can show them (rows are NOT part of the parent payload).
pub async fn get_task_body_impl(
    notion_page_id: &str,
    token: &str,
) -> anyhow::Result<Vec<serde_json::Value>> {
    let mut blocks = fetch_block_children(notion_page_id, token).await?;
    for b in &mut blocks {
        let is_table = b["type"].as_str() == Some("table");
        let has_children = b["has_children"].as_bool() == Some(true);
        if is_table && has_children {
            if let Some(id) = b["id"].as_str() {
                let id = id.to_string();
                match fetch_block_children(&id, token).await {
                    Ok(rows) => b["__children"] = serde_json::Value::Array(rows),
                    Err(e) => tracing::warn!("table rows fetch failed for {id}: {e}"),
                }
            }
        }
    }
    Ok(blocks)
}

/// Fetch the task template as markdown, validating the configured id first —
/// a database id (or an inaccessible/archived page) must fail with a clear,
/// actionable error instead of handing junk blocks to the agent.
pub async fn get_task_template_markdown(page_id: &str, token: &str) -> anyhow::Result<String> {
    if let Err(e) = notion_get(token, &format!("v1/pages/{page_id}")).await {
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

/// One rich-text span → markdown, honoring Notion's inline annotations and link.
/// Dropping these was why bold/code in a ticket body arrived as flat text (and
/// why the UI, which renders this markdown, showed nothing special).
fn span_to_markdown(span: &serde_json::Value) -> String {
    let text = span["plain_text"].as_str().unwrap_or("");
    if text.is_empty() {
        return String::new();
    }
    let ann = &span["annotations"];
    let flag = |k: &str| ann[k].as_bool() == Some(true);

    // Code first: markdown doesn't nest emphasis inside a code span.
    let mut out = if flag("code") {
        // Use a longer fence when the text itself contains backticks.
        let ticks = if text.contains('`') { "``" } else { "`" };
        format!("{ticks}{text}{ticks}")
    } else {
        text.to_string()
    };
    if flag("bold") {
        out = format!("**{out}**");
    }
    if flag("italic") {
        out = format!("_{out}_");
    }
    if flag("strikethrough") {
        out = format!("~~{out}~~");
    }
    if let Some(href) = span["href"].as_str().filter(|h| !h.is_empty()) {
        // Keep surrounding whitespace outside the brackets — otherwise the anchor
        // text starts with a space, which renders as an oddly padded link.
        let trimmed = out.trim();
        let lead = &out[..out.len() - out.trim_start().len()];
        let trail = &out[out.trim_end().len()..];
        out = format!("{lead}[{trimmed}]({href}){trail}");
    }
    out
}

/// All spans of a block's rich text, as markdown.
fn rich_to_markdown(rich: &serde_json::Value) -> String {
    rich.as_array()
        .map(|arr| arr.iter().map(span_to_markdown).collect())
        .unwrap_or_default()
}

/// Render Notion blocks as markdown — the shape the agent both reads (task
/// template) and writes back (`create_task_from_explorer`'s `body_markdown`),
/// and what the task overview renders.
pub fn blocks_to_markdown(blocks: &[serde_json::Value]) -> String {
    fn plain(block: &serde_json::Value, ty: &str) -> String {
        rich_to_markdown(&block[ty]["rich_text"])
    }
    let mut out: Vec<String> = vec![];
    let mut numbered = 0u32;
    for b in blocks {
        let ty = b["type"].as_str().unwrap_or("");
        if ty != "numbered_list_item" {
            numbered = 0;
        }
        match ty {
            "heading_1" => out.push(format!("# {}", plain(b, ty))),
            "heading_2" => out.push(format!("## {}", plain(b, ty))),
            "heading_3" => out.push(format!("### {}", plain(b, ty))),
            "bulleted_list_item" => out.push(format!("- {}", plain(b, ty))),
            "numbered_list_item" => {
                numbered += 1;
                out.push(format!("{numbered}. {}", plain(b, ty)));
            }
            "to_do" => {
                let checked = b["to_do"]["checked"].as_bool() == Some(true);
                out.push(format!("- [{}] {}", if checked { "x" } else { " " }, plain(b, ty)));
            }
            "quote" | "callout" => out.push(format!("> {}", plain(b, ty))),
            "divider" => out.push("---".to_string()),
            "code" => {
                let lang = b["code"]["language"].as_str().unwrap_or("");
                // Raw text inside a fence — markdown escaping would be literal here.
                let body: String = b[ty]["rich_text"]
                    .as_array()
                    .map(|arr| arr.iter().filter_map(|t| t["plain_text"].as_str()).collect())
                    .unwrap_or_default();
                out.push(format!("```{lang}\n{body}\n```"));
            }
            "table" => {
                // Rows arrive as `__children` (attached by get_task_body_impl).
                let Some(rows) = b["__children"].as_array() else { continue };
                for (ri, row) in rows.iter().enumerate() {
                    let cells: Vec<String> = row["table_row"]["cells"]
                        .as_array()
                        .map(|cs| {
                            cs.iter()
                                // Cells keep their inline formatting; a literal pipe
                                // would break the GFM row, so escape it.
                                .map(|cell| rich_to_markdown(cell).replace('|', "\\|"))
                                .collect()
                        })
                        .unwrap_or_default();
                    if cells.is_empty() {
                        continue;
                    }
                    out.push(format!("| {} |", cells.join(" | ")));
                    // GFM needs the separator after the first row.
                    if ri == 0 {
                        out.push(format!("|{}|", vec![" --- "; cells.len()].join("|")));
                    }
                }
            }
            "paragraph" => {
                let text = plain(b, ty);
                out.push(text); // empty paragraph = blank line, preserves spacing
            }
            _ => {
                // Unknown block types degrade to their text content, if any.
                let text = plain(b, ty);
                if !text.is_empty() {
                    out.push(text);
                }
            }
        }
    }
    out.join("\n")
}

/// Resolve a database's title property name (the property whose type == "title").
async fn resolve_title_prop(token: &str, database_id: &str) -> anyhow::Result<String> {
    Ok(super::schema::load(token, database_id).await?.title_property)
}

/// The sprint database, read off the Sprint relation rather than configured: the
/// property already says where it points. `None` when the task database has no
/// sprint relation, in which case there is nothing to filter by.
pub(super) async fn sprint_database_id(
    token: &str,
    database_id: &str,
    sprint_prop: &str,
) -> Option<String> {
    super::schema::load(token, database_id)
        .await
        .ok()?
        .relation_target(sprint_prop)
        .map(str::to_string)
}

/// Create a page in the tasks database. Returns `(notion_page_id, short_id)`,
/// where `short_id` is read back from the auto-generated unique_id property.
#[allow(clippy::too_many_arguments)]
pub(super) async fn create_task_page(
    token: &str,
    database_id: &str,
    title: &str,
    status_prop: &str,
    status_value: &str,
    assignee_prop: Option<&str>,
    user_id: &str,
    sprint_prop: Option<&str>,
    sprint_ids: &[String],
    project_prop: Option<&str>,
    project_id: Option<&str>,
    mut children: Vec<serde_json::Value>,
) -> anyhow::Result<(String, String)> {
    let title_prop = resolve_title_prop(token, database_id).await?;

    let mut properties = serde_json::Map::new();
    properties.insert(title_prop, serde_json::json!({ "title": [{ "text": { "content": title } }] }));
    if !status_value.is_empty() {
        properties.insert(status_prop.to_string(), serde_json::json!({ "status": { "name": status_value } }));
    }
    if let Some(ap) = assignee_prop {
        if !user_id.is_empty() {
            properties.insert(ap.to_string(), serde_json::json!({ "people": [{ "id": user_id }] }));
        }
    }
    if let Some(sp) = sprint_prop {
        if !sprint_ids.is_empty() {
            let rel: Vec<_> = sprint_ids.iter().map(|id| serde_json::json!({ "id": id })).collect();
            properties.insert(sp.to_string(), serde_json::json!({ "relation": rel }));
        }
    }
    if let (Some(pp), Some(pid)) = (project_prop, project_id) {
        properties.insert(pp.to_string(), serde_json::json!({ "relation": [{ "id": pid }] }));
    }

    // Notion caps `children` at 100 blocks on page create — send the first 100
    // with the create and append the rest in follow-up batches.
    let (first, rest) = if children.len() > 100 {
        let rest = children.split_off(100);
        (children, rest)
    } else {
        (children, vec![])
    };

    let body = serde_json::json!({
        "parent": { "database_id": database_id },
        "properties": properties,
        "children": first,
    });

    let page = notion_post(token, "v1/pages", &body).await?;
    let notion_page_id = page["id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("created page missing id"))?
        .to_string();
    let short_id = extract_unique_id(&page["properties"])
        .ok_or_else(|| anyhow::anyhow!("created page has no unique_id — is that property configured on the DB?"))?;

    for chunk in rest.chunks(100) {
        notion_patch(
            token,
            &format!("v1/blocks/{notion_page_id}/children"),
            &serde_json::json!({ "children": chunk }),
        )
        .await?;
    }

    Ok((notion_page_id, short_id))
}

/// Strip a `"<digits>. "` numbered-list prefix, returning the remainder.
fn strip_numbered(s: &str) -> Option<&str> {
    let rest = s.trim_start_matches(|c: char| c.is_ascii_digit());
    if rest.len() < s.len() && rest.starts_with(". ") {
        Some(&rest[2..])
    } else {
        None
    }
}

/// Notion rejects any single rich_text content over 2000 chars — split long
/// text into multiple text objects.
fn rich(text: &str) -> serde_json::Value {
    const MAX: usize = 2000;
    if text.len() <= MAX {
        return serde_json::json!([{ "type": "text", "text": { "content": text } }]);
    }
    let mut parts = vec![];
    let mut rest = text;
    while !rest.is_empty() {
        // Split on a char boundary at or below MAX bytes.
        let mut cut = MAX.min(rest.len());
        while !rest.is_char_boundary(cut) {
            cut -= 1;
        }
        let (head, tail) = rest.split_at(cut);
        parts.push(serde_json::json!({ "type": "text", "text": { "content": head } }));
        rest = tail;
    }
    serde_json::Value::Array(parts)
}

/// Map a markdown fence language to one Notion's code block accepts (it rejects
/// unknown values outright, which would fail the whole page create).
fn notion_code_language(lang: &str) -> &'static str {
    match lang.trim().to_ascii_lowercase().as_str() {
        "rust" | "rs" => "rust",
        "python" | "py" => "python",
        "typescript" | "ts" | "tsx" => "typescript",
        "javascript" | "js" | "jsx" => "javascript",
        "go" | "golang" => "go",
        "json" => "json",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "sql" => "sql",
        "html" => "html",
        "css" => "css",
        "c" => "c",
        "cpp" | "c++" => "c++",
        "java" => "java",
        "ruby" | "rb" => "ruby",
        "bash" | "sh" | "shell" | "zsh" => "shell",
        "diff" => "diff",
        "markdown" | "md" => "markdown",
        _ => "plain text",
    }
}

/// Minimal Markdown → Notion blocks converter for drafted task bodies. Handles
/// headings, paragraphs, bullet/numbered lists, to-dos, quotes, dividers, and
/// fenced code blocks.
pub(super) fn markdown_to_blocks(md: &str) -> Vec<serde_json::Value> {
    let mut blocks = vec![];
    // Fenced code accumulator: Some((language, lines)) while inside ``` … ```.
    let mut fence: Option<(String, Vec<String>)> = None;

    let flush_fence = |fence: &mut Option<(String, Vec<String>)>, blocks: &mut Vec<serde_json::Value>| {
        if let Some((lang, lines)) = fence.take() {
            blocks.push(serde_json::json!({
                "object": "block",
                "type": "code",
                "code": {
                    "rich_text": rich(&lines.join("\n")),
                    "language": notion_code_language(&lang),
                }
            }));
        }
    };

    for raw in md.lines() {
        if fence.is_some() {
            if raw.trim_start().starts_with("```") {
                flush_fence(&mut fence, &mut blocks);
            } else if let Some((_, lines)) = fence.as_mut() {
                lines.push(raw.to_string());
            }
            continue;
        }
        let trimmed = raw.trim();
        if let Some(lang) = trimmed.strip_prefix("```") {
            fence = Some((lang.to_string(), vec![]));
            continue;
        }
        if trimmed.is_empty() {
            continue;
        }
        let block = if let Some(t) = trimmed.strip_prefix("### ") {
            serde_json::json!({ "object":"block","type":"heading_3","heading_3":{"rich_text": rich(t)} })
        } else if let Some(t) = trimmed.strip_prefix("## ") {
            serde_json::json!({ "object":"block","type":"heading_2","heading_2":{"rich_text": rich(t)} })
        } else if let Some(t) = trimmed.strip_prefix("# ") {
            serde_json::json!({ "object":"block","type":"heading_1","heading_1":{"rich_text": rich(t)} })
        } else if trimmed == "---" {
            serde_json::json!({ "object":"block","type":"divider","divider":{} })
        } else if let Some(t) = trimmed.strip_prefix("> ") {
            serde_json::json!({ "object":"block","type":"quote","quote":{"rich_text": rich(t)} })
        } else if let Some(t) = trimmed.strip_prefix("- [ ] ") {
            serde_json::json!({ "object":"block","type":"to_do","to_do":{"rich_text": rich(t), "checked": false} })
        } else if let Some(t) = trimmed.strip_prefix("- [x] ") {
            serde_json::json!({ "object":"block","type":"to_do","to_do":{"rich_text": rich(t), "checked": true} })
        } else if let Some(t) = trimmed.strip_prefix("- ").or_else(|| trimmed.strip_prefix("* ")) {
            serde_json::json!({ "object":"block","type":"bulleted_list_item","bulleted_list_item":{"rich_text": rich(t)} })
        } else if let Some(t) = strip_numbered(trimmed) {
            serde_json::json!({ "object":"block","type":"numbered_list_item","numbered_list_item":{"rich_text": rich(t)} })
        } else {
            serde_json::json!({ "object":"block","type":"paragraph","paragraph":{"rich_text": rich(trimmed)} })
        };
        blocks.push(block);
    }
    // Unterminated fence: still emit what was collected.
    flush_fence(&mut fence, &mut blocks);
    blocks
}

#[cfg(test)]
mod tests {
    use super::blocks_to_markdown;

    fn span(text: &str, ann: serde_json::Value, href: Option<&str>) -> serde_json::Value {
        serde_json::json!({ "plain_text": text, "annotations": ann, "href": href })
    }

    #[test]
    fn inline_annotations_survive_the_markdown_round_trip() {
        let none = serde_json::json!({ "bold": false, "italic": false, "code": false, "strikethrough": false });
        let bold = serde_json::json!({ "bold": true, "italic": false, "code": false, "strikethrough": false });
        let code = serde_json::json!({ "bold": false, "italic": false, "code": true, "strikethrough": false });

        let blocks = vec![serde_json::json!({
            "type": "paragraph",
            "paragraph": { "rich_text": [
                span("set ", none.clone(), None),
                span("retries", code, None),
                span(" to ", none.clone(), None),
                span("3", bold, None),
                span(" see docs", none, Some("https://example.com")),
            ]}
        })];

        assert_eq!(
            blocks_to_markdown(&blocks),
            "set `retries` to **3** [see docs](https://example.com)"
        );
    }

    #[test]
    fn code_blocks_stay_raw_and_tables_escape_pipes() {
        let plain = serde_json::json!({ "bold": false, "italic": false, "code": false, "strikethrough": false });
        let blocks = vec![
            serde_json::json!({
                "type": "code",
                "code": { "language": "rust", "rich_text": [span("let x = 1;", plain.clone(), None)] }
            }),
            serde_json::json!({
                "type": "table",
                "table": {},
                "__children": [{ "table_row": { "cells": [
                    [span("a|b", plain.clone(), None)],
                    [span("c", plain, None)]
                ]}}]
            }),
        ];
        let md = blocks_to_markdown(&blocks);
        assert!(md.contains("```rust\nlet x = 1;\n```"), "got: {md}");
        assert!(md.contains("| a\\|b | c |"), "got: {md}");
    }
}
