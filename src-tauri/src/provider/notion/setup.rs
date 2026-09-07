//! Notion's half of setup: the setup payload and the config built from it.

use crate::core::config::{FilterConfig, NotionConfig, REDACTED};
use crate::core::error::{AppError, AppResult, ErrorKind};

#[derive(serde::Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct NotionSetup {
    pub token: String,
    pub database_id: String,
    pub user_id: String,
    pub template_page_id: Option<String>,
}

/// Manual so no `{:?}` can print the token.
impl std::fmt::Debug for NotionSetup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NotionSetup")
            .field("token", &REDACTED)
            .field("database_id", &self.database_id)
            .field("user_id", &self.user_id)
            .field("template_page_id", &self.template_page_id)
            .finish()
    }
}

/// The detected schema, for the setup screen.
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
    /// Every status option.
    pub status_options: Vec<String>,
}

/// Read the database's vocabulary. Also the check that the integration can see it.
#[tauri::command]
pub async fn detect_notion_database(
    token: String,
    database_id: String,
) -> AppResult<DetectedSchema> {
    let schema = super::schema::load(&token, database_id.trim())
        .await
        .map_err(|e| {
            AppError::new(
                ErrorKind::Provider,
                format!("Cannot read that database: {e}"),
            )
        })?;
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

/// The Notion config from the setup payload. Reading the schema also checks
/// that the integration can see the database.
pub async fn build_config(n: &NotionSetup) -> AppResult<NotionConfig> {
    let token = n.token.trim();
    let database_id = n.database_id.trim();
    if token.is_empty() || database_id.is_empty() {
        return Err(AppError::invalid(
            "A Notion token and database id are both required.",
        ));
    }

    let schema = super::schema::load(token, database_id).await.map_err(|e| {
        AppError::new(
            ErrorKind::Provider,
            format!("Notion rejected the database: {e}"),
        )
    })?;
    let properties = super::detect::detect_properties(&schema);
    let status_map = super::detect::detect_status_map(&schema);

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
    if let Some(id) = &template {
        super::body::template_markdown(id, token)
            .await
            .map_err(|e| {
                AppError::new(
                    ErrorKind::Provider,
                    format!("That template page could not be read: {e}"),
                )
            })?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_output_never_carries_the_token() {
        let setup = NotionSetup {
            token: "ntn_secret".into(),
            database_id: "db".into(),
            user_id: "user".into(),
            template_page_id: None,
        };
        let rendered = format!("{setup:?}");
        assert!(
            !rendered.contains("ntn_secret"),
            "token reached a Debug rendering: {rendered}"
        );
        assert!(
            rendered.contains(REDACTED),
            "no redaction marker: {rendered}"
        );
    }
}
