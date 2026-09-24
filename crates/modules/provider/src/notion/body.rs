//! A page's blocks as the text the overview shows.

/// Every block's own text, one to a line.
pub(super) fn text(reply: &serde_json::Value) -> String {
    let blocks = reply["results"].as_array().cloned().unwrap_or_default();
    blocks
        .iter()
        .map(line)
        .collect::<Vec<String>>()
        .join("\n")
        .trim()
        .to_string()
}

/// One block: its kind's own rich text, with what marks a list item.
fn line(block: &serde_json::Value) -> String {
    let kind = block["type"].as_str().unwrap_or_default();
    let words = runs(&block[kind]["rich_text"]);
    match kind {
        "bulleted_list_item" | "numbered_list_item" => format!("- {words}"),
        "to_do" => match block["to_do"]["checked"].as_bool() {
            Some(true) => format!("- [x] {words}"),
            _ => format!("- [ ] {words}"),
        },
        _ => words,
    }
}

fn runs(value: &serde_json::Value) -> String {
    value
        .as_array()
        .map(|runs| {
            runs.iter()
                .filter_map(|run| run["plain_text"].as_str())
                .collect::<String>()
        })
        .unwrap_or_default()
}
