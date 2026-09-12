//! The markdown <-> Notion-blocks bridge, both directions. Pure functions, no I/O.

/// One rich-text span → markdown, honoring Notion's inline annotations and link.
fn span_to_markdown(span: &serde_json::Value) -> String {
    let text = span["plain_text"].as_str().unwrap_or("");
    if text.is_empty() {
        return String::new();
    }
    let ann = &span["annotations"];
    let flag = |k: &str| ann[k].as_bool() == Some(true);

    // Code span first, then emphasis.
    let mut out = if flag("code") {
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
        // Surrounding whitespace stays outside the brackets.
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

/// Render Notion blocks as markdown.
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
                out.push(format!(
                    "- [{}] {}",
                    if checked { "x" } else { " " },
                    plain(b, ty)
                ));
            }
            "quote" | "callout" => out.push(format!("> {}", plain(b, ty))),
            "divider" => out.push("---".to_string()),
            "code" => {
                let lang = b["code"]["language"].as_str().unwrap_or("");
                // Plain text inside a fence, no inline markdown.
                let body: String = b[ty]["rich_text"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|t| t["plain_text"].as_str())
                            .collect()
                    })
                    .unwrap_or_default();
                out.push(format!("```{lang}\n{body}\n```"));
            }
            "table" => {
                // Rows arrive as `__children` (attached by body::get_task_body_impl).
                let Some(rows) = b["__children"].as_array() else {
                    continue;
                };
                for (ri, row) in rows.iter().enumerate() {
                    let cells: Vec<String> = row["table_row"]["cells"]
                        .as_array()
                        .map(|cs| {
                            cs.iter()
                                .map(|cell| rich_to_markdown(cell).replace('|', "\\|"))
                                .collect()
                        })
                        .unwrap_or_default();
                    if cells.is_empty() {
                        continue;
                    }
                    out.push(format!("| {} |", cells.join(" | ")));
                    // Header separator row.
                    if ri == 0 {
                        out.push(format!("|{}|", vec![" --- "; cells.len()].join("|")));
                    }
                }
            }
            "paragraph" => {
                let text = plain(b, ty);
                out.push(text); // empty paragraph = blank line
            }
            _ => {
                // Unknown block types degrade to their text content.
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

/// Notion's per-span content limit.
const SPAN_MAX: usize = 2000;

/// Literal text as rich_text spans of at most 2000 chars each. For code blocks:
/// their content carries no inline markup.
fn rich_plain(text: &str) -> serde_json::Value {
    let mut parts = vec![];
    push_text(&mut parts, text, Marks::default(), None);
    serde_json::Value::Array(parts)
}

/// The inline annotations a span can carry, as `span_to_markdown` writes them.
#[derive(Clone, Copy, Default)]
struct Marks {
    bold: bool,
    italic: bool,
    code: bool,
    strikethrough: bool,
}

impl Marks {
    fn to_json(self) -> serde_json::Value {
        serde_json::json!({
            "bold": self.bold,
            "italic": self.italic,
            "code": self.code,
            "strikethrough": self.strikethrough,
        })
    }
}

/// Append `text` as spans, split on a char boundary at or below the span limit.
fn push_text(out: &mut Vec<serde_json::Value>, text: &str, marks: Marks, href: Option<&str>) {
    if text.is_empty() {
        return;
    }
    let mut rest = text;
    while !rest.is_empty() {
        let mut cut = SPAN_MAX.min(rest.len());
        while !rest.is_char_boundary(cut) {
            cut -= 1;
        }
        let (head, tail) = rest.split_at(cut);
        out.push(serde_json::json!({
            "type": "text",
            "text": { "content": head, "link": href.map(|h| serde_json::json!({ "url": h })) },
            "annotations": marks.to_json(),
        }));
        rest = tail;
    }
}

/// The closing delimiter's byte offset in `text`, or None when it is unmatched.
fn find_close(text: &str, delim: &str) -> Option<usize> {
    let mut from = 0;
    while let Some(i) = text[from..].find(delim) {
        let at = from + i;
        // `_` and `*` never close on an empty span, so `**` is not read as two italics.
        if at > 0 {
            return Some(at);
        }
        from = at + delim.len();
    }
    None
}

/// Inline markdown as rich_text spans, inverting `span_to_markdown`: links,
/// `~~strike~~`, `**bold**`, `_italic_` and `` `code` ``. An unmatched
/// delimiter stays literal.
fn rich_inline(text: &str) -> serde_json::Value {
    let mut out = vec![];
    parse_inline(text, Marks::default(), None, &mut out);
    serde_json::Value::Array(out)
}

fn parse_inline(text: &str, marks: Marks, href: Option<&str>, out: &mut Vec<serde_json::Value>) {
    let mut literal_from = 0;
    let mut i = 0;
    let bytes = text.as_bytes();

    while i < text.len() {
        if !text.is_char_boundary(i) {
            i += 1;
            continue;
        }
        let rest = &text[i..];

        // A code span's content is literal, so it is tried before emphasis.
        let code_delim = if rest.starts_with("``") {
            Some("``")
        } else if rest.starts_with('`') {
            Some("`")
        } else {
            None
        };
        if !marks.code {
            if let Some(delim) = code_delim {
                let after = &rest[delim.len()..];
                if let Some(end) = find_close(after, delim) {
                    push_text(out, &text[literal_from..i], marks, href);
                    let mut inner = marks;
                    inner.code = true;
                    push_text(out, &after[..end], inner, href);
                    i += delim.len() + end + delim.len();
                    literal_from = i;
                    continue;
                }
            }
        }

        // A link wraps the marks, so its label is parsed with the marks in force.
        if href.is_none() && bytes[i] == b'[' {
            if let Some((label, url, len)) = split_link(rest) {
                push_text(out, &text[literal_from..i], marks, href);
                parse_inline(label, marks, Some(url), out);
                i += len;
                literal_from = i;
                continue;
            }
        }

        let emphasis = [("~~", 's'), ("**", 'b'), ("_", 'i'), ("*", 'i')]
            .into_iter()
            .find(|(delim, kind)| {
                rest.starts_with(delim)
                    && match kind {
                        'b' => !marks.bold,
                        'i' => !marks.italic,
                        _ => !marks.strikethrough,
                    }
            });
        if let Some((delim, kind)) = emphasis {
            let after = &rest[delim.len()..];
            if let Some(end) = find_close(after, delim) {
                push_text(out, &text[literal_from..i], marks, href);
                let mut inner = marks;
                match kind {
                    'b' => inner.bold = true,
                    'i' => inner.italic = true,
                    _ => inner.strikethrough = true,
                }
                parse_inline(&after[..end], inner, href, out);
                i += delim.len() + end + delim.len();
                literal_from = i;
                continue;
            }
        }

        i += 1;
    }
    push_text(out, &text[literal_from..], marks, href);
}

/// `[label](url)` at the start of `text`, as (label, url, consumed bytes).
fn split_link(text: &str) -> Option<(&str, &str, usize)> {
    let close = text.find("](")?;
    let label = &text[1..close];
    if label.contains('[') || label.contains(']') {
        return None;
    }
    let after = &text[close + 2..];
    let end = after.find(')')?;
    let url = &after[..end];
    if url.is_empty() {
        return None;
    }
    Some((label, url, close + 2 + end + 1))
}

/// A markdown fence language as one Notion's code block accepts.
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

/// Markdown -> Notion blocks: headings, paragraphs, lists, to-dos, quotes,
/// dividers and fenced code.
pub fn markdown_to_blocks(md: &str) -> Vec<serde_json::Value> {
    let mut blocks = vec![];
    // Some((language, lines)) while inside a fence.
    let mut fence: Option<(String, Vec<String>)> = None;

    let flush_fence = |fence: &mut Option<(String, Vec<String>)>,
                       blocks: &mut Vec<serde_json::Value>| {
        if let Some((lang, lines)) = fence.take() {
            blocks.push(serde_json::json!({
                "object": "block",
                "type": "code",
                "code": {
                    "rich_text": rich_plain(&lines.join("\n")),
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
            serde_json::json!({ "object":"block","type":"heading_3","heading_3":{"rich_text": rich_inline(t)} })
        } else if let Some(t) = trimmed.strip_prefix("## ") {
            serde_json::json!({ "object":"block","type":"heading_2","heading_2":{"rich_text": rich_inline(t)} })
        } else if let Some(t) = trimmed.strip_prefix("# ") {
            serde_json::json!({ "object":"block","type":"heading_1","heading_1":{"rich_text": rich_inline(t)} })
        } else if trimmed == "---" {
            serde_json::json!({ "object":"block","type":"divider","divider":{} })
        } else if let Some(t) = trimmed.strip_prefix("> ") {
            serde_json::json!({ "object":"block","type":"quote","quote":{"rich_text": rich_inline(t)} })
        } else if let Some(t) = trimmed.strip_prefix("- [ ] ") {
            serde_json::json!({ "object":"block","type":"to_do","to_do":{"rich_text": rich_inline(t), "checked": false} })
        } else if let Some(t) = trimmed.strip_prefix("- [x] ") {
            serde_json::json!({ "object":"block","type":"to_do","to_do":{"rich_text": rich_inline(t), "checked": true} })
        } else if let Some(t) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
        {
            serde_json::json!({ "object":"block","type":"bulleted_list_item","bulleted_list_item":{"rich_text": rich_inline(t)} })
        } else if let Some(t) = strip_numbered(trimmed) {
            serde_json::json!({ "object":"block","type":"numbered_list_item","numbered_list_item":{"rich_text": rich_inline(t)} })
        } else {
            serde_json::json!({ "object":"block","type":"paragraph","paragraph":{"rich_text": rich_inline(trimmed)} })
        };
        blocks.push(block);
    }
    // Flush an unterminated fence.
    flush_fence(&mut fence, &mut blocks);
    blocks
}

#[cfg(test)]
mod tests {
    use super::{blocks_to_markdown, markdown_to_blocks};

    /// `plain_text` and `href` are what Notion returns; the writer sends `text.content`
    /// and `text.link`. Read one back as the other so a round trip can be asserted.
    fn as_notion_read(blocks: Vec<serde_json::Value>) -> Vec<serde_json::Value> {
        let mut out = blocks;
        for b in out.iter_mut() {
            let ty = b["type"].as_str().unwrap_or("").to_string();
            let Some(spans) = b[&ty]["rich_text"].as_array().cloned() else {
                continue;
            };
            let read: Vec<serde_json::Value> = spans
                .into_iter()
                .map(|s| {
                    serde_json::json!({
                        "plain_text": s["text"]["content"],
                        "annotations": s["annotations"],
                        "href": s["text"]["link"]["url"],
                    })
                })
                .collect();
            b[&ty]["rich_text"] = serde_json::Value::Array(read);
        }
        out
    }

    #[test]
    fn inline_marks_survive_a_write_and_read_back() {
        for md in [
            "set `retries` to **3** [see docs](https://example.com)",
            "# A **bold** heading",
            "- a _slanted_ item",
            "- [x] done with ~~a change~~",
            "> quoting `code` and **bold**",
            "plain text with no marks",
        ] {
            let round = blocks_to_markdown(&as_notion_read(markdown_to_blocks(md)));
            assert_eq!(round, md, "round trip changed the text");
        }
    }

    /// Nested marks keep their annotations. The text is not byte-stable: the writer
    /// emits one delimiter pair per span, so the reader sees `**a **_**b**_`.
    #[test]
    fn nested_marks_annotate_every_span() {
        let blocks = markdown_to_blocks("a **bold _and slanted_ run**");
        let spans = blocks[0]["paragraph"]["rich_text"].as_array().unwrap();
        let marked: Vec<(&str, bool, bool)> = spans
            .iter()
            .map(|s| {
                (
                    s["text"]["content"].as_str().unwrap(),
                    s["annotations"]["bold"].as_bool().unwrap(),
                    s["annotations"]["italic"].as_bool().unwrap(),
                )
            })
            .collect();
        assert_eq!(
            marked,
            vec![
                ("a ", false, false),
                ("bold ", true, false),
                ("and slanted", true, true),
                (" run", true, false),
            ]
        );
    }

    #[test]
    fn an_unmatched_delimiter_stays_literal() {
        let blocks = markdown_to_blocks("a * b and _ c and ` d");
        let spans = blocks[0]["paragraph"]["rich_text"].as_array().unwrap();
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0]["text"]["content"], "a * b and _ c and ` d");
        assert_eq!(spans[0]["annotations"]["italic"], false);
    }

    #[test]
    fn a_fence_keeps_its_markup_literal() {
        let blocks = markdown_to_blocks("```rust\nlet a = **b** * c;\n```");
        let spans = blocks[0]["code"]["rich_text"].as_array().unwrap();
        assert_eq!(spans[0]["text"]["content"], "let a = **b** * c;");
        assert_eq!(spans[0]["annotations"]["bold"], false);
    }

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
