//! A GraphQL list read past its first page: its cursor, and the pages put after it.

use serde_json::Value;

/// The most pages of a list one read takes.
pub(crate) const PAGES_MAX: usize = 10;

/// What a list asks for to give its pages.
pub(crate) const PAGE_INFO: &str = "pageInfo { hasNextPage endCursor }";

/// The cursor a list's next page starts after, when it has one.
pub(crate) fn after(list: &Value) -> Option<String> {
    let info = &list["pageInfo"];
    let more = info["hasNextPage"].as_bool() == Some(true);
    info["endCursor"]
        .as_str()
        .filter(|_| more)
        .map(String::from)
}

/// One more page put at the end of the list.
pub(crate) fn extend(list: &mut Value, page: &Value) {
    let more = page["nodes"].as_array().cloned().unwrap_or_default();
    if let Some(held) = list["nodes"].as_array_mut() {
        held.extend(more);
    }
    list["pageInfo"] = page["pageInfo"].clone();
}
