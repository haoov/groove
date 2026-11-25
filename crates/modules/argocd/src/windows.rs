//! The sync windows an Application's project puts on it, as Argo CD answers for the Application.

use groove_types::{SyncWindow, SyncWindows};
use serde_json::Value;

/// The windows of the Application `namespace/name`.
pub fn windows_path(namespace: &str, name: &str) -> String {
    format!("/api/v1/applications/{name}/syncwindows?appNamespace={namespace}")
}

/// Its windows; `Err` holds what Argo CD said when the body is none.
pub fn windows(body: &str) -> Result<SyncWindows, String> {
    let json: Value = serde_json::from_str(body).map_err(|_| first_line(body))?;
    let Some(can_sync) = json["canSync"].as_bool() else {
        let said = json["message"].as_str().map(String::from);
        return Err(said.unwrap_or_else(|| first_line(body)));
    };
    Ok(SyncWindows {
        can_sync,
        assigned: listed(&json["assignedWindows"]),
        active: listed(&json["activeWindows"]),
    })
}

fn listed(all: &Value) -> Vec<SyncWindow> {
    let all = all.as_array().map(Vec::as_slice).unwrap_or_default();
    all.iter().map(window).collect()
}

fn window(one: &Value) -> SyncWindow {
    let text = |key: &str| one[key].as_str().unwrap_or_default().to_string();
    SyncWindow {
        kind: text("kind"),
        schedule: text("schedule"),
        duration: text("duration"),
        time_zone: text("timeZone"),
        manual: one["manualSync"].as_bool().unwrap_or_default(),
    }
}

fn first_line(body: &str) -> String {
    body.lines().next().unwrap_or_default().to_string()
}
