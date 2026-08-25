//! Notion's half of setup: what the user fills in, and the config built from it.
//!
//! Lives with the provider — task_manager/setup.rs stays source-agnostic and
//! only assembles whichever sources were filled in.

use crate::core::config::{FilterConfig, NotionConfig};

#[derive(Debug, serde::Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct NotionSetup {
    pub token: String,
    pub database_id: String,
    pub user_id: String,
    pub template_page_id: Option<String>,
}

/// What the database says about itself, for the setup screen to show before
/// saving.
///
/// The point is that the user can SEE what was detected: a silent wrong guess
/// about which property holds the status is worse than a visible one.
#[derive(Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct DetectedSchema {
    pub title_property: String,
    pub status_property: String,
    pub priority_property: Option<String>,
    pub sprint_property: Option<String>,
    pub project_property: Option<String>,
    pub assignee_property: Option<String>,
    /// The status values the app will write when filing / starting / finishing.
    pub status_ready: String,
    pub status_in_progress: String,
    pub status_done: String,
    /// Every status option, so a wrong pick is obvious in context.
    pub status_options: Vec<String>,
}

/// Read the database's vocabulary. Also the check that the integration can see it.
#[tauri::command]
pub async fn detect_notion_database(
    token: String,
    database_id: String,
) -> Result<DetectedSchema, String> {
    let schema = super::schema::load(&token, database_id.trim())
        .await
        .map_err(|e| format!("Cannot read that database: {e}"))?;
    let props = super::detect::detect_properties(&schema);
    let status = super::detect::detect_status_map(&schema);
    Ok(DetectedSchema {
        title_property: schema.title_property.clone(),
        status_property: props.status.clone(),
        priority_property: props.priority.clone(),
        sprint_property: props.sprint.clone(),
        project_property: props.project.clone(),
        assignee_property: props.assignee.clone(),
        status_ready: status.ready,
        status_in_progress: status.in_progress,
        status_done: status.done,
        status_options: schema
            .properties
            .iter()
            .find(|p| p.name == props.status)
            .map(|p| p.options.iter().map(|o| o.title.clone()).collect())
            .unwrap_or_default(),
    })
}

/// Reading the schema is both the detection and the check that the integration
/// can see this database — the most likely mistake, and one that would otherwise
/// surface later as an empty task list.
pub async fn build_config(n: &NotionSetup) -> Result<NotionConfig, String> {
    let token = n.token.trim();
    let database_id = n.database_id.trim();
    if token.is_empty() || database_id.is_empty() {
        return Err("A Notion token and database id are both required.".into());
    }

    let schema = super::schema::load(token, database_id)
        .await
        .map_err(|e| format!("Notion rejected the database: {e}"))?;
    let properties = super::detect::detect_properties(&schema);
    let status_map = super::detect::detect_status_map(&schema);

    // Excluding the completion state is what keeps finished work off Home. Detected
    // rather than assumed to be called "Done".
    let exclude_statuses = if status_map.done.is_empty() {
        vec![]
    } else {
        vec![status_map.done.clone()]
    };

    let template = n
        .template_page_id
        .as_ref()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty());
    // Validate it now: a template id that cannot be read fails at explorer→task
    // conversion, long after setup, with nothing pointing back here.
    if let Some(id) = &template {
        super::body::template_markdown(id, token)
            .await
            .map_err(|e| format!("That template page could not be read: {e}"))?;
    }

    let user_id = n.user_id.trim().to_string();
    Ok(NotionConfig {
        token: token.to_string(),
        database_id: database_id.to_string(),
        filters: FilterConfig {
            exclude_statuses,
            filter_by_assignee: !user_id.is_empty(),
        },
        user_id,
        properties,
        status_map,
        task_template_page_id: template,
        default_project_id: None,
    })
}
