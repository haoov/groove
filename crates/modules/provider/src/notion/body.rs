//! A page's blocks as Markdown.

/// How many levels under the page's own blocks are read.
pub(crate) const DEEP: usize = 2;

/// A block and the blocks nested under it.
#[derive(Debug, Clone, Default)]
pub(crate) struct Node {
    pub block: serde_json::Value,
    pub children: Vec<Node>,
}

/// Whether the blocks under this one, `depth` levels down, are read.
pub(crate) fn opens(block: &serde_json::Value, depth: usize) -> bool {
    let kind = block["type"].as_str().unwrap_or_default();
    let page = matches!(kind, "child_page" | "child_database");
    depth < DEEP && block["has_children"].as_bool() == Some(true) && !page
}

/// Every block as Markdown, a blank line between blocks and none inside a list.
pub(crate) fn text(nodes: &[Node]) -> String {
    let mut said = Vec::new();
    flatten(nodes, "", &mut said);
    let mut out = String::new();
    let mut listed = false;
    for (block, item) in said {
        if !out.is_empty() {
            out.push_str(if item && listed { "\n" } else { "\n\n" });
        }
        out.push_str(&block);
        listed = item;
    }
    out.trim().to_string()
}

/// The blocks in order, each with whether it is a list item; a list item's children stand inside it.
fn flatten(nodes: &[Node], indent: &str, out: &mut Vec<(String, bool)>) {
    for node in nodes {
        let Some(said) = block_of(&node.block) else {
            continue;
        };
        let item = is_item(&node.block);
        let lines: Vec<String> = said.lines().map(|line| format!("{indent}{line}")).collect();
        out.push((lines.join("\n"), item));
        let inner = match (item, said.starts_with("1. ")) {
            (true, true) => format!("{indent}   "),
            (true, false) => format!("{indent}  "),
            (false, _) => indent.to_string(),
        };
        flatten(&node.children, &inner, out);
    }
}

fn is_item(block: &serde_json::Value) -> bool {
    matches!(
        block["type"].as_str(),
        Some("bulleted_list_item" | "numbered_list_item" | "to_do")
    )
}

/// One block in its Markdown form, or nothing for a kind that holds no text.
fn block_of(block: &serde_json::Value) -> Option<String> {
    let kind = block["type"].as_str()?;
    let body = &block[kind];
    let words = || runs(&body["rich_text"]);
    Some(match kind {
        "heading_1" => format!("# {}", words()),
        "heading_2" => format!("## {}", words()),
        "heading_3" => format!("### {}", words()),
        "bulleted_list_item" => format!("- {}", words()),
        "numbered_list_item" => format!("1. {}", words()),
        "to_do" => match body["checked"].as_bool() {
            Some(true) => format!("- [x] {}", words()),
            _ => format!("- [ ] {}", words()),
        },
        "quote" | "callout" => format!("> {}", words().replace('\n', "\n> ")),
        "code" => format!("```{}\n{}\n```", language(body), plain(&body["rich_text"])),
        "divider" => "---".to_string(),
        "paragraph" | "toggle" => words(),
        _ => return None,
    })
}

fn language(body: &serde_json::Value) -> &str {
    match body["language"].as_str() {
        Some("plain text") | None => "",
        Some(one) => one,
    }
}

/// The runs' own text, no Markdown added.
fn plain(value: &serde_json::Value) -> String {
    let runs = value.as_array().into_iter().flatten();
    runs.filter_map(|run| run["plain_text"].as_str()).collect()
}

/// The runs with their bold, italic, code and links as Markdown; a line break stays one.
fn runs(value: &serde_json::Value) -> String {
    let runs = value.as_array().into_iter().flatten();
    let marked: String = runs.map(run).collect();
    marked.replace('\n', "  \n")
}

fn run(run: &serde_json::Value) -> String {
    let text = run["plain_text"].as_str().unwrap_or_default();
    let core = text.trim();
    if core.is_empty() {
        return text.to_string();
    }
    let marks = &run["annotations"];
    let on = |name: &str| marks[name].as_bool().unwrap_or(false);
    let mut said = core.to_string();
    if on("code") {
        said = format!("`{said}`");
    }
    if on("italic") {
        said = format!("*{said}*");
    }
    if on("bold") {
        said = format!("**{said}**");
    }
    if on("strikethrough") {
        said = format!("~~{said}~~");
    }
    if let Some(href) = run["href"].as_str() {
        said = format!("[{said}]({href})");
    }
    let (start, end) = (text.len() - text.trim_start().len(), text.trim_end().len());
    format!("{}{said}{}", &text[..start], &text[end..])
}
