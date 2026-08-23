//! First-run discovery: which boards exist, and what one looks like.

use crate::core::forge::api;
use crate::provider::detect::detect_status_map;
use crate::task_manager::DetectedSchema;

const DEFAULT_HOST: &str = "github.com";

const BOARDS: &str = r#"
query {
  viewer {
    login
    projectsV2(first: 50) { nodes { id title number closed } }
    organizations(first: 20) {
      nodes { login projectsV2(first: 50) { nodes { id title number closed } } }
    }
  }
}
"#;

#[derive(Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct GithubProject {
    pub id: String,
    pub title: String,
    #[ts(type = "number")]
    pub number: i64,
    /// Which account it belongs to, since two can share a title.
    pub owner: String,
    /// Closed boards are listed but not offered first — an archived board has no
    /// queue worth syncing.
    pub closed: bool,
}

/// Every Projects v2 board the signed-in user can see.
#[tauri::command]
pub async fn list_github_projects(host: Option<String>) -> Result<Vec<GithubProject>, String> {
    let host = host.unwrap_or_else(|| DEFAULT_HOST.to_string());
    let res = api::github_graphql(&host, BOARDS, serde_json::json!({}))
        .await
        .map_err(|e| e.to_string())?;

    let viewer = &res["data"]["viewer"];
    let login = viewer["login"].as_str().unwrap_or_default().to_string();
    let mut out = collect(&viewer["projectsV2"], &login);

    for org in viewer["organizations"]["nodes"].as_array().unwrap_or(&vec![]) {
        let owner = org["login"].as_str().unwrap_or_default().to_string();
        out.extend(collect(&org["projectsV2"], &owner));
    }

    out.sort_by_key(|p| (p.closed, p.owner.clone(), -p.number));
    Ok(out)
}

fn collect(projects: &serde_json::Value, owner: &str) -> Vec<GithubProject> {
    projects["nodes"]
        .as_array()
        .map(|n| {
            n.iter()
                .filter_map(|p| {
                    Some(GithubProject {
                        id: p["id"].as_str()?.to_string(),
                        title: p["title"].as_str()?.to_string(),
                        number: p["number"].as_i64().unwrap_or_default(),
                        owner: owner.to_string(),
                        closed: p["closed"].as_bool().unwrap_or(false),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// What the app would read off a board, for the setup screen to show before saving.
#[tauri::command]
pub async fn detect_github_project(
    project_id: String,
    host: Option<String>,
) -> Result<DetectedSchema, String> {
    let host = host.unwrap_or_else(|| DEFAULT_HOST.to_string());
    let cfg = crate::core::config::GithubConfig {
        host,
        projects: vec![],
        properties: crate::core::config::GithubPropertyNames {
            status: "Status".into(),
            priority: None,
            iteration: None,
        },
        status_map: crate::core::config::StatusMap {
            ready: String::new(),
            in_progress: String::new(),
            done: String::new(),
        },
        filters: crate::core::config::FilterConfig {
            exclude_statuses: vec![],
            filter_by_assignee: true,
        },
    };

    let schema = super::schema::board_schema(&cfg, &project_id)
        .await
        .map_err(|e| format!("That board could not be read: {e}"))?;
    let status = detect_status_map(&schema);

    let named = |hint: &str| {
        schema
            .properties
            .iter()
            .find(|p| p.name.eq_ignore_ascii_case(hint))
            .map(|p| p.name.clone())
    };
    let status_property = named("Status").unwrap_or_else(|| "Status".into());

    Ok(DetectedSchema {
        title_property: "Title".into(),
        status_options: schema
            .properties
            .iter()
            .find(|p| p.name == status_property)
            .map(|p| p.options.iter().map(|o| o.title.clone()).collect())
            .unwrap_or_default(),
        status_property,
        priority_property: named("Priority"),
        // An iteration IS a sprint; the label reads correctly either way.
        sprint_property: schema
            .properties
            .iter()
            .find(|p| p.name.eq_ignore_ascii_case("Iteration") || p.name.eq_ignore_ascii_case("Sprint"))
            .map(|p| p.name.clone()),
        // A board spans repos; there is no project relation to detect.
        project_property: None,
        assignee_property: named("Assignees"),
        status_ready: status.ready,
        status_in_progress: status.in_progress,
        status_done: status.done,
    })
}
