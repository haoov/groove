//! Reading a forge's or a source's JSON reply: a missing field reads as nothing.

use crate::Timestamp;

/// The string at `value`, or empty.
pub fn text(value: &serde_json::Value) -> String {
    value.as_str().unwrap_or_default().to_string()
}

/// The time the string at `value` says.
pub fn at(value: &serde_json::Value) -> Option<Timestamp> {
    Timestamp::parse(value.as_str()?).ok()
}

/// The `nodes` of a GraphQL list.
pub fn nodes(list: &serde_json::Value) -> Vec<serde_json::Value> {
    list["nodes"].as_array().cloned().unwrap_or_default()
}
