//! Reading a source's vocabulary instead of asking for it.
//!
//! Type-first with a name tiebreaker, never a name guess: a source with `Assignee`
//! and `Reporter` (both people) needs the name to pick, but one that calls it
//! `Owner` still resolves, because it is the only people property.
//!
//! The result is written to the config, where it can be corrected. Detection that
//! cannot be overridden is just a different hardcoding.

use super::types::{StatusGroup, TaskSchema};
use crate::core::config::StatusMap;

/// Lowercase, letters and digits only: makes "To-do", "to_do" and "To Do" the same
/// string, so group names from either API surface compare equal.
pub(super) fn norm(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).flat_map(|c| c.to_lowercase()).collect()
}

/// Number fields hours are logged into, matched case-insensitively — a board or
/// database is named by hand and "Time spent" is as likely as "Hours". ONE list
/// for every provider: two copies had already drifted apart.
const HOURS_NAMES: [&str; 4] = ["Hours spent", "Hours", "Time spent", "Time spent (H)"];

/// Whether `name` is where hours are logged. Exact names only (normalized): a
/// contains-match would silently write into "Hours estimate".
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

/// The three status values the app actually writes: what it sets when a task is
/// filed (`ready`), picked up (`in_progress`) and finished (`done`).
///
/// Group membership decides the meaning; the name only chooses within a group. In a
/// real database "Complete" holds `Fixed with required action`, `Done`, `Abandoned`
/// and `Archived` — all completions, but only one of them is what finishing a task
/// should set.
pub fn detect_status_map(schema: &TaskSchema) -> StatusMap {
    let g = &schema.status_groups;
    let all: Vec<String> = schema
        .properties
        .iter()
        .find(|p| p.kind == "status")
        .map(|p| p.options.iter().map(|o| o.title.clone()).collect())
        .unwrap_or_default();

    // No groups (a `select` used as a status, or an API surface that omits them):
    // fall back to matching over every option.
    let from = |group: &str, hint: &str| -> Option<String> {
        let scoped = group_options(g, group);
        if scoped.is_empty() { pick(&all, hint) } else { pick(scoped, hint) }
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

    /// Case and separators must not matter; near-misses must.
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
