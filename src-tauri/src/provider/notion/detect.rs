//! Detection of property names from the database schema: type first, name as
//! tiebreaker. The result is written to the config, where the user can correct it.

use super::schema::TaskSchema;
use crate::core::config::PropertyNames;

pub use crate::provider::detect::detect_status_map;
use crate::provider::detect::norm;

/// The first property of one of `kinds`, preferring a name containing `hint`.
fn find(schema: &TaskSchema, kinds: &[&str], hint: &str) -> Option<String> {
    let candidates: Vec<&crate::provider::types::PropertySchema> = schema
        .properties
        .iter()
        .filter(|p| kinds.contains(&p.kind.as_str()))
        .collect();
    let hint = norm(hint);
    candidates
        .iter()
        .find(|p| norm(&p.name) == hint)
        .or_else(|| candidates.iter().find(|p| norm(&p.name).contains(&hint)))
        .or_else(|| candidates.first())
        .map(|p| p.name.clone())
}

/// Same, but only when the name matches.
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
        status: find(schema, &["status"], "status").unwrap_or_else(|| "Status".to_string()),
        priority: find_named(schema, &["select", "status"], "priority"),
        sprint: find_named(schema, &["relation"], "sprint"),
        project: find_named(schema, &["relation"], "project"),
        assignee: find(schema, &["people", "person"], "assignee"),
    }
}

#[cfg(test)]
mod tests {
    use super::super::schema::PropertySchema;
    use super::*;
    use crate::provider::types::StatusGroup;

    fn prop(name: &str, kind: &str, options: &[&str]) -> PropertySchema {
        PropertySchema {
            meta: false,
            name: name.into(),
            kind: kind.into(),
            options: options
                .iter()
                .map(|o| crate::provider::types::PropertyOption::named(*o))
                .collect(),
            relation_db: None,
            editable: true,
        }
    }

    fn group(name: &str, options: &[&str]) -> StatusGroup {
        StatusGroup {
            name: name.into(),
            options: options.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// Two people properties, three relations, four completions in one group.
    fn real() -> TaskSchema {
        TaskSchema {
            hours_property: None,
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
                prop(
                    "Status",
                    "status",
                    &[
                        "To be defined",
                        "Ready for sprint",
                        "In progress",
                        "Blocked",
                        "Fixed with required action",
                        "Done",
                        "Abandoned",
                        "Archived",
                    ],
                ),
            ],
            status_groups: vec![
                group("To-do", &["To be defined", "Ready for sprint"]),
                group("In progress", &["In progress", "Blocked"]),
                group(
                    "Complete",
                    &[
                        "Fixed with required action",
                        "Done",
                        "Abandoned",
                        "Archived",
                    ],
                ),
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
        assert_eq!(p.assignee.as_deref(), Some("Assignee"));
    }

    #[test]
    fn reads_the_real_status_values_from_their_groups() {
        let m = detect_status_map(&real());
        assert_eq!(m.ready, "Ready for sprint");
        assert_eq!(m.in_progress, "In progress");
        assert_eq!(m.done, "Done");
    }

    #[test]
    fn resolves_a_database_that_uses_other_words() {
        let schema = TaskSchema {
            hours_property: None,
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
        assert_eq!(
            p.status, "State",
            "the only status property, whatever it is called"
        );
        assert_eq!(
            p.assignee.as_deref(),
            Some("Owner"),
            "the only people property"
        );
        assert_eq!(p.priority, None, "absent means absent, not a wrong guess");
        assert_eq!(p.sprint, None, "a relation called Cycle is not a sprint");

        let m = detect_status_map(&schema);
        assert_eq!(m.ready, "Backlog");
        assert_eq!(m.in_progress, "Doing");
        assert_eq!(m.done, "Shipped");
    }

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

    #[test]
    fn falls_back_to_every_option_when_there_are_no_groups() {
        let schema = TaskSchema {
            hours_property: None,
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
            hours_property: None,
            database_id: "db".into(),
            title_property: "Name".into(),
            properties: vec![prop("Name", "title", &[])],
            status_groups: vec![],
        };
        let p = detect_properties(&schema);
        assert_eq!(
            p.status, "Status",
            "the conventional name is the only fallback"
        );
        assert_eq!(p.assignee, None);
        let m = detect_status_map(&schema);
        assert!(m.ready.is_empty() && m.in_progress.is_empty() && m.done.is_empty());
    }
}
