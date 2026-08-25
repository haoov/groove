//! The markdown ↔ Notion-blocks bridge, both directions. Pure functions, no I/O.
//!
//! Markdown is the app's lingua franca: the agent reads and writes it, the UI
//! renders it, and Notion's inline annotations survive the round trip.

/// One rich-text span → markdown, honoring Notion's inline annotations and link.
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
/// template) and writes back, and what the task overview renders.
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
                // Rows arrive as `__children` (attached by body::get_task_body_impl).
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
pub fn markdown_to_blocks(md: &str) -> Vec<serde_json::Value> {
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
