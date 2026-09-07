//! Detection of a source's status vocabulary and hours field from its schema.
//! The result is written to the config, where the user can correct it.

use super::types::{StatusGroup, TaskSchema};
use crate::core::config::StatusMap;

/// Lowercase, letters and digits only: "To-do", "to_do" and "To Do" compare equal.
pub(super) fn norm(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// Number fields hours are logged into. The one list for every provider.
const HOURS_NAMES: [&str; 4] = ["Hours spent", "Hours", "Time spent", "Time spent (H)"];

/// Whether `name` is where hours are logged. Exact normalized match only, never contains.
pub(super) fn is_hours_property(name: &str) -> bool {
    HOURS_NAMES.iter().any(|h| norm(h) == norm(name))
}

/// Options in the group whose name normalizes to `group`.
fn group_options<'a>(groups: &'a [StatusGroup], group: &str) -> &'a [String] {
    groups
        .iter()
        .find(|g| norm(&g.name) == group)
        .map(|g| g.options.as_slice())
        .unwrap_or(&[])
}

/// The option in `options` whose name contains `hint`, else the first one.
fn pick(options: &[String], hint: &str) -> Option<String> {
    let hint = norm(hint);
    options
        .iter()
        .find(|o| norm(o) == hint)
        .or_else(|| options.iter().find(|o| norm(o).contains(&hint)))
        .or_else(|| options.first())
        .cloned()
}

/// The three status values the app writes: `ready`, `in_progress`, `done`.
/// Group membership decides the meaning; the name only chooses within a group.
pub fn detect_status_map(schema: &TaskSchema) -> StatusMap {
    let g = &schema.status_groups;
    let all: Vec<String> = schema
        .properties
        .iter()
        .find(|p| p.kind == "status")
        .map(|p| p.options.iter().map(|o| o.title.clone()).collect())
        .unwrap_or_default();

    // Without groups, match over every option.
    let from = |group: &str, hint: &str| -> Option<String> {
        let scoped = group_options(g, group);
        if scoped.is_empty() {
            pick(&all, hint)
        } else {
            pick(scoped, hint)
        }
    };

    StatusMap {
        ready: from("todo", "ready").unwrap_or_default(),
        in_progress: from("inprogress", "progress").unwrap_or_default(),
        done: from("complete", "done").unwrap_or_default(),
    }
}

#[cfg(test)]
mod hours_tests {
    use super::is_hours_property;

    #[test]
    fn hours_names_match_exactly_but_loosely() {
        for yes in ["Hours spent", "hours SPENT", "Time spent (h)", "Hours"] {
            assert!(is_hours_property(yes), "{yes}");
        }
        for no in ["Hours estimate", "Spent", "Time"] {
            assert!(!is_hours_property(no), "{no}");
        }
    }
}
