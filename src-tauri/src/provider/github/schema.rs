//! Board field definitions as a TaskSchema.

use crate::core::config::GithubConfig;
use crate::core::forge::api;
use crate::provider::types::{PropertyOption, PropertySchema, StatusGroup, TaskSchema};

/// Board columns the app renders from the issue itself, not as editable fields.
const READ_ONLY: [&str; 9] = [
    "Title",
    "Assignees",
    "Labels",
    "Repository",
    "Milestone",
    "Reviewers",
    "Linked pull requests",
    "Parent issue",
    "Sub-issues progress",
];
/// Number fields hours are logged into, matched case-insensitively — a board is
/// named by hand and "Time spent" is as likely as "Hours".
const HOURS_NAMES: [&str; 4] = ["Hours spent", "Hours", "Time spent", "Time spent (H)"];

/// Board columns GitHub maintains itself.
const TIMESTAMPS: [&str; 3] = ["Created", "Updated", "Closed"];

const FIELDS: &str = r#"
query($project: ID!) {
  node(id: $project) {
    ... on ProjectV2 {
      title
      fields(first: 50) {
        nodes {
          ... on ProjectV2FieldCommon { __typename name dataType }
          ... on ProjectV2SingleSelectField { options { id name } }
          ... on ProjectV2IterationField {
            configuration { iterations { id title } completedIterations { id title } }
          }
        }
      }
    }
  }
}
"#;

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
    let res =
        api::github_graphql(&cfg.host, FIELDS, serde_json::json!({ "project": project_id })).await?;
    let node = &res["data"]["node"];

    let mut properties: Vec<PropertySchema> = node["fields"]["nodes"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|f| {
            let name = f["name"].as_str()?.to_string();
            let data_type = f["dataType"].as_str().unwrap_or("TEXT");
            let kind = kind_of(data_type).to_string();

            let mut options: Vec<PropertyOption> = f["options"]
                .as_array()
                .map(|o| {
                    o.iter()
                        .filter_map(|x| {
                            Some(PropertyOption {
                                id: x["id"].as_str()?.to_string(),
                                title: x["name"].as_str()?.to_string(),
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();
            // An iteration's "options" are its iterations; completed ones stay
            // listed so an existing value still resolves to a name.
            for key in ["iterations", "completedIterations"] {
                if let Some(iters) = f["configuration"][key].as_array() {
                    options.extend(iters.iter().filter_map(|i| {
                        Some(PropertyOption {
                            id: i["id"].as_str()?.to_string(),
                            title: i["title"].as_str()?.to_string(),
                        })
                    }));
                }
            }

            let meta = TIMESTAMPS.contains(&name.as_str()) || READ_ONLY.contains(&name.as_str());
            Some(PropertySchema {
                editable: !meta,
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
        .find(|p| p.kind == "number" && HOURS_NAMES.iter().any(|h| p.name.eq_ignore_ascii_case(h)))
        .map(|p| p.name.clone());

    Ok(TaskSchema {
        database_id: project_id.to_string(),
        title_property: "Title".to_string(),
        properties,
        status_groups,
        hours_property,
    })
}
