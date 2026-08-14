//! Reading the database's vocabulary instead of asking for it.
//!
//! Property names and status values were configuration, which meant a new user had
//! to describe their own database to the app and any mismatch showed up later as a
//! property that "does not exist". Notion already knows all of it: every property
//! carries a type, and a status property carries GROUPS — Notion's own To-do /
//! In progress / Complete classification of its options.
//!
//! Detection is therefore type-first with a name tiebreaker, never a name guess:
//! a database with `Assignee` and `Reporter` (both people) needs the name to pick,
//! but a database that calls it `Owner` still resolves, because it is the only
//! people property.
//!
//! The result is still written to the config, where it can be corrected. Detection
//! that cannot be overridden is just a different hardcoding.

use super::config::{PropertyNames, StatusMap};
use super::schema::{StatusGroup, TaskSchema};

/// Lowercase, letters and digits only: makes "To-do", "to_do" and "To Do" the same
/// string, so group names from either API surface compare equal.
fn norm(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).flat_map(|c| c.to_lowercase()).collect()
}

/// The first property of one of `kinds`, preferring a name containing `hint`.
///
/// Type narrows, the name only breaks ties. Notion's public API calls a person
/// property `people`; other surfaces call it `person`, so callers pass both.
fn find(schema: &TaskSchema, kinds: &[&str], hint: &str) -> Option<String> {
    let candidates: Vec<&super::schema::PropertySchema> =
        schema.properties.iter().filter(|p| kinds.contains(&p.kind.as_str())).collect();
    let hint = norm(hint);
    candidates
        .iter()
        .find(|p| norm(&p.name) == hint)
        .or_else(|| candidates.iter().find(|p| norm(&p.name).contains(&hint)))
        .or_else(|| candidates.first())
        .map(|p| p.name.clone())
}

/// Same, but only when the name matches: with three relations (Sprint, Project,
/// Platform Components) "the first relation" would be a coin toss.
fn find_named(schema: &TaskSchema, kinds: &[&str], hint: &str) -> Option<String> {
    let hint = norm(hint);
    schema
        .properties
        .iter()
        .filter(|p| kinds.contains(&p.kind.as_str()))
        .find(|p| norm(&p.name).contains(&hint))
        .map(|p| p.name.clone())
}

/// Which property holds what, read off the schema.
pub fn detect_properties(schema: &TaskSchema) -> PropertyNames {
    PropertyNames {
        // A database without a status property is not usable, so fall back to the
        // conventional name rather than inventing one.
        status: find(schema, &["status"], "status").unwrap_or_else(|| "Status".to_string()),
        priority: find_named(schema, &["select", "status"], "priority"),
        sprint: find_named(schema, &["relation"], "sprint"),
        project: find_named(schema, &["relation"], "project"),
        assignee: find(schema, &["people", "person"], "assignee"),
    }
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
        .map(|p| p.options.clone())
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
mod tests {
    use super::*;
    use super::super::schema::PropertySchema;

    fn prop(name: &str, kind: &str, options: &[&str]) -> PropertySchema {
        PropertySchema {
            name: name.into(),
            kind: kind.into(),
            options: options.iter().map(|s| s.to_string()).collect(),
            relation_db: None,
            editable: true,
        }
    }

    fn group(name: &str, options: &[&str]) -> StatusGroup {
        StatusGroup { name: name.into(), options: options.iter().map(|s| s.to_string()).collect() }
    }

    /// The real Platform Tasks database: two people properties, three relations,
    /// and a Complete group holding four different completions.
    fn real() -> TaskSchema {
        TaskSchema {
            database_id: "db".into(),
            title_property: "Task name".into(),
            properties: vec![
                prop("Assignee", "people", &[]),
                prop("Reporter", "people", &[]),
                prop("Priority", "select", &["Low", "Medium", "High"]),
                prop("Sprint", "relation", &[]),
                prop("Project", "relation", &[]),
                prop("Platform Components", "relation", &[]),
                prop("Task name", "title", &[]),
                prop("Status", "status", &[
                    "To be defined", "Ready for sprint", "In progress", "Blocked",
                    "Fixed with required action", "Done", "Abandoned", "Archived",
                ]),
            ],
            status_groups: vec![
                group("To-do", &["To be defined", "Ready for sprint"]),
                group("In progress", &["In progress", "Blocked"]),
                group("Complete", &[
                    "Fixed with required action", "Done", "Abandoned", "Archived",
                ]),
            ],
        }
    }

    #[test]
    fn finds_every_property_in_the_real_database() {
        let p = detect_properties(&real());
        assert_eq!(p.status, "Status");
        assert_eq!(p.priority.as_deref(), Some("Priority"));
        assert_eq!(p.sprint.as_deref(), Some("Sprint"));
        assert_eq!(p.project.as_deref(), Some("Project"));
        // Two people properties: the name has to break the tie, or tasks get
        // assigned to the Reporter.
        assert_eq!(p.assignee.as_deref(), Some("Assignee"));
    }

    #[test]
    fn reads_the_real_status_values_from_their_groups() {
        let m = detect_status_map(&real());
        assert_eq!(m.ready, "Ready for sprint");
        assert_eq!(m.in_progress, "In progress");
        // NOT "Fixed with required action", which sorts first in the group.
        assert_eq!(m.done, "Done");
    }

    /// A differently-named database must still resolve: type first, name second.
    #[test]
    fn resolves_a_database_that_uses_other_words() {
        let schema = TaskSchema {
            database_id: "db".into(),
            title_property: "Name".into(),
            properties: vec![
                prop("Owner", "people", &[]),
                prop("Name", "title", &[]),
                prop("State", "status", &["Backlog", "Doing", "Shipped"]),
                prop("Cycle", "relation", &[]),
            ],
            status_groups: vec![
                group("To-do", &["Backlog"]),
                group("In progress", &["Doing"]),
                group("Complete", &["Shipped"]),
            ],
        };
        let p = detect_properties(&schema);
        assert_eq!(p.status, "State", "the only status property, whatever it is called");
        assert_eq!(p.assignee.as_deref(), Some("Owner"), "the only people property");
        assert_eq!(p.priority, None, "absent means absent, not a wrong guess");
        assert_eq!(p.sprint, None, "a relation called Cycle is not a sprint");

        let m = detect_status_map(&schema);
        assert_eq!(m.ready, "Backlog");
        assert_eq!(m.in_progress, "Doing");
        assert_eq!(m.done, "Shipped");
    }

    /// Group names differ between API surfaces (`To-do` vs `to_do`).
    #[test]
    fn accepts_either_spelling_of_a_group_name() {
        let mut schema = real();
        schema.status_groups = vec![
            group("to_do", &["To be defined", "Ready for sprint"]),
            group("in_progress", &["In progress"]),
            group("complete", &["Done"]),
        ];
        let m = detect_status_map(&schema);
        assert_eq!(m.ready, "Ready for sprint");
        assert_eq!(m.in_progress, "In progress");
        assert_eq!(m.done, "Done");
    }

    /// A `select` standing in for a status has no groups at all.
    #[test]
    fn falls_back_to_every_option_when_there_are_no_groups() {
        let schema = TaskSchema {
            database_id: "db".into(),
            title_property: "Name".into(),
            properties: vec![
                prop("Name", "title", &[]),
                prop("Status", "status", &["Ready", "In progress", "Done"]),
            ],
            status_groups: vec![],
        };
        let m = detect_status_map(&schema);
        assert_eq!(m.ready, "Ready");
        assert_eq!(m.in_progress, "In progress");
        assert_eq!(m.done, "Done");
    }

    #[test]
    fn an_empty_database_produces_empty_values_not_wrong_ones() {
        let schema = TaskSchema {
            database_id: "db".into(),
            title_property: "Name".into(),
            properties: vec![prop("Name", "title", &[])],
            status_groups: vec![],
        };
        let p = detect_properties(&schema);
        assert_eq!(p.status, "Status", "the conventional name is the only fallback");
        assert_eq!(p.assignee, None);
        let m = detect_status_map(&schema);
        assert!(m.ready.is_empty() && m.in_progress.is_empty() && m.done.is_empty());
    }
}
