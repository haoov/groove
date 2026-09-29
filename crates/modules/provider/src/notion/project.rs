//! The project a page belongs to: a select's name, or the title of the first page a relation names.

use std::collections::HashMap;
use std::sync::Mutex;

/// What the project property holds.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Held {
    Named(String),
    Page(String),
}

pub(super) fn held(page: &serde_json::Value, name: Option<&str>) -> Option<Held> {
    let value = &page["properties"][name?];
    match value["type"].as_str()? {
        "select" => value["select"]["name"]
            .as_str()
            .map(|one| Held::Named(one.to_string())),
        "relation" => value["relation"][0]["id"]
            .as_str()
            .map(|one| Held::Page(one.to_string())),
        _ => None,
    }
}

/// The titles of the project pages read so far, by page id.
#[derive(Debug, Default)]
pub(super) struct Titles {
    held: Mutex<HashMap<String, String>>,
}

impl Titles {
    pub(super) fn get(&self, id: &str) -> Option<String> {
        self.held.lock().ok()?.get(id).cloned()
    }

    pub(super) fn keep(&self, id: &str, title: &str) {
        if let Ok(mut held) = self.held.lock() {
            held.insert(id.to_string(), title.to_string());
        }
    }
}
