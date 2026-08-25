//! First-run preview: what the app would actually pick up.

const DEFAULT_HOST: &str = "github.com";

#[derive(Debug, serde::Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct GithubSetup {
    /// github.com unless a GitHub Enterprise host is given.
    pub host: Option<String>,
}

/// Nothing to validate: there is no board to nominate, and gh already holds the
/// credential. The field names are Projects v2's own defaults and are corrected
/// in the config file if a board names them differently. Reconnecting keeps
/// `existing` — the hand-corrected values live nowhere else.
pub fn build_config(
    g: &GithubSetup,
    existing: Option<crate::core::config::GithubConfig>,
) -> crate::core::config::GithubConfig {
    if let Some(existing) = existing {
        return existing;
    }
    crate::core::config::GithubConfig {
        host: g.host.clone().unwrap_or_else(|| DEFAULT_HOST.to_string()),
        properties: crate::core::config::GithubPropertyNames {
            status: "Status".into(),
            priority: Some("Priority".into()),
        },
        // Each board names its own columns, so a write reads them off the board
        // and only falls back to this.
        status_map: crate::core::config::StatusMap {
            ready: "Ready".into(),
            in_progress: "In progress".into(),
            done: "Done".into(),
        },
    }
}

/// What GitHub would give the queue right now, so the setup screen can show it
/// before anything is saved. Nothing to configure — this is the whole answer.
#[derive(Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct GithubPreview {
    /// Open issues assigned to you that sit on a board.
    #[ts(type = "number")]
    pub tasks: i64,
    /// The boards they came from, so an unexpected one is visible.
    pub boards: Vec<String>,
    /// Board field names the app found, for the same reason.
    pub fields: Vec<String>,
    /// Assigned issues on no board, which are deliberately not tasks.
    #[ts(type = "number")]
    pub unboarded: i64,
    /// Each board's Status columns. Boards name their states freely, and a wrong
    /// status_map makes Finish fail — show what is really there.
    pub status_columns: Vec<BoardColumns>,
}

#[derive(Debug, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct BoardColumns {
    pub board: String,
    pub columns: Vec<String>,
}

#[tauri::command]
pub async fn preview_github(host: Option<String>) -> Result<GithubPreview, String> {
    let host = host.unwrap_or_else(|| DEFAULT_HOST.to_string());

    let items = super::projects::assigned_issues(&host).await.map_err(|e| e.to_string())?;

    let mut boards: Vec<String> = items.iter().map(|i| i.board.clone()).collect();
    boards.sort();
    boards.dedup();

    let mut fields: Vec<String> =
        items.iter().flat_map(|i| i.fields.iter().map(|f| f.name.clone())).collect();
    fields.sort();
    fields.dedup();

    let unboarded = super::projects::assigned_count(&host).await.unwrap_or(0) - items.len() as i64;

    // One row per board the tasks came from, deduplicated by project id.
    let mut seen: Vec<(String, String)> = vec![];
    for i in &items {
        if !seen.iter().any(|(id, _)| id == &i.project_id) {
            seen.push((i.project_id.clone(), i.board.clone()));
        }
    }
    let mut status_columns = Vec::with_capacity(seen.len());
    for (project_id, board) in seen {
        let columns = super::fields::status_columns(&host, &project_id).await;
        status_columns.push(BoardColumns { board, columns });
    }

    Ok(GithubPreview {
        tasks: items.len() as i64,
        boards,
        fields,
        unboarded: unboarded.max(0),
        status_columns,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_github_config_defaults_and_a_reconnect_keeps_corrections() {
        let fresh = build_config(&GithubSetup { host: None }, None);
        assert_eq!(fresh.host, "github.com");
        assert_eq!(fresh.properties.status, "Status");

        // Hand-corrected values live only in the config file — keep them.
        let mut corrected = fresh.clone();
        corrected.status_map.done = "Shipped".to_string();
        let kept = build_config(&GithubSetup { host: None }, Some(corrected));
        assert_eq!(kept.status_map.done, "Shipped");
    }
}
