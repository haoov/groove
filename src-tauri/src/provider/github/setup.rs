//! First-run preview: what the app would actually pick up.

const DEFAULT_HOST: &str = "github.com";

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

    Ok(GithubPreview {
        tasks: items.len() as i64,
        boards,
        fields,
        unboarded: unboarded.max(0),
    })
}
