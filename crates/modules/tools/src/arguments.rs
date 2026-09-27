//! A tool's arguments, read by name.

use serde_json::Value;

pub trait Arguments {
    /// One argument as a string; an empty one is none.
    fn text(&self, name: &str) -> Option<&str>;
    fn number(&self, name: &str) -> Option<i64>;
    fn flag(&self, name: &str) -> Option<bool>;
}

impl Arguments for Value {
    fn text(&self, name: &str) -> Option<&str> {
        self[name].as_str().filter(|one| !one.is_empty())
    }

    fn number(&self, name: &str) -> Option<i64> {
        self[name].as_i64()
    }

    fn flag(&self, name: &str) -> Option<bool> {
        self[name].as_bool()
    }
}
