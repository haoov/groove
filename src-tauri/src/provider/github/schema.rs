//! Board field definitions as a TaskSchema.

use crate::core::config::GithubConfig;
use crate::provider::types::{PropertyOption, PropertySchema, StatusGroup, TaskSchema};

/// Columns that come off the issue rather than the board, so they are shown but
/// cannot be set here. `properties()` fills their values.
const DISPLAY_ONLY: [&str; 2] = ["Labels", "Assignees"];

/// Not fields at all: the title is the row, and the rest is board plumbing.
const NOT_A_FIELD: [&str; 7] = [
    "Title",
    "Repository",
    "Milestone",
    "Reviewers",
    "Linked pull requests",
    "Parent issue",
    "Sub-issues progress",
];
/// Board columns GitHub maintains itself.
const TIMESTAMPS: [&str; 3] = ["Created", "Updated", "Closed"];


/// GitHub's dataType, in the shared vocabulary.
fn kind_of(data_type: &str) -> &'static str {
    match data_type {
        "SINGLE_SELECT" => "select",
        "NUMBER" => "number",
        "DATE" => "date",
        "ITERATION" => "select",
        _ => "text",
    }
}

pub(super) async fn board_schema(
    cfg: &GithubConfig,
    project_id: &str,
) -> anyhow::Result<TaskSchema> {
    let nodes = super::fields::board_fields(&cfg.host, project_id).await?;

    let mut properties: Vec<PropertySchema> = nodes
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|f| {
            let name = f["name"].as_str()?.to_string();
            let data_type = f["dataType"].as_str().unwrap_or("TEXT");
            let kind = kind_of(data_type).to_string();

            // `id` is the value the app sends back, not the node id: a write
            // resolves the option id from the name (see fields::field_value), the
            // same shape Notion uses.
            let mut options: Vec<PropertyOption> = f["options"]
                .as_array()
                .map(|o| {
                    o.iter()
                        .filter_map(|x| Some(PropertyOption::named(x["name"].as_str()?)))
                        .collect()
                })
                .unwrap_or_default();
            // An iteration's "options" are its iterations; completed ones stay
            // listed so an existing value still resolves to a name.
            for key in ["iterations", "completedIterations"] {
                if let Some(iters) = f["configuration"][key].as_array() {
                    options.extend(
                        iters.iter().filter_map(|i| Some(PropertyOption::named(i["title"].as_str()?))),
                    );
                }
            }

            let display_only = DISPLAY_ONLY.contains(&name.as_str());
            let meta = TIMESTAMPS.contains(&name.as_str()) || NOT_A_FIELD.contains(&name.as_str());
            // The board's Status column is a status, not a plain select: the strip
            // gives that kind the coloured dot and the tone.
            let kind = match name.eq_ignore_ascii_case("Status") && kind == "select" {
                true => "status".to_string(),
                false => kind,
            };
            Some(PropertySchema {
                editable: !meta && !display_only,
                meta,
                name,
                kind,
                options,
                relation_db: None,
            })
        })
        .collect();
    properties.sort_by(|a, b| a.name.cmp(&b.name));

    // The board's Status options are a flat list — Projects v2 has no equivalent of
    // Notion's To-do / In progress / Complete grouping, so detection falls back to
    // matching over every option.
    let status_groups: Vec<StatusGroup> = vec![];

    let hours_property = properties
        .iter()
        .find(|p| p.kind == "number" && crate::provider::detect::is_hours_property(&p.name))
        .map(|p| p.name.clone());

    Ok(TaskSchema {
        database_id: project_id.to_string(),
        title_property: "Title".to_string(),
        properties,
        status_groups,
        hours_property,
    })
}
