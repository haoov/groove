//! What differs between an Application's desired state and its live one, as Argo CD's managed resources say.

use serde_json::Value;

/// One resource that differs: its live side and its desired side, either missing.
#[derive(Debug, Clone, PartialEq)]
pub struct Drifted {
    pub group: String,
    pub kind: String,
    pub namespace: Option<String>,
    pub name: String,
    /// Live, as Argo CD normalizes it; none when the resource is missing live.
    pub live: Option<Value>,
    /// Desired, as Argo CD predicts it live; none when a sync would prune it.
    pub desired: Option<Value>,
}

/// The managed resources of the Application `namespace/name`.
pub fn drift_path(namespace: &str, name: &str) -> String {
    format!("/api/v1/applications/{name}/managed-resources?appNamespace={namespace}")
}

/// Each resource that differs, hooks left out; `Err` holds what Argo CD said when the body is no list.
pub fn drift(body: &str) -> Result<Vec<Drifted>, String> {
    let json: Value = serde_json::from_str(body).map_err(|_| first_line(body))?;
    let Some(items) = json["items"].as_array() else {
        let said = json["message"].as_str().map(String::from);
        return Err(said.unwrap_or_else(|| first_line(body)));
    };
    Ok(items.iter().filter_map(drifted).collect())
}

fn drifted(item: &Value) -> Option<Drifted> {
    if item["hook"].as_bool() == Some(true) {
        return None;
    }
    let live = state(item, "normalizedLiveState").or_else(|| state(item, "liveState"));
    let desired = state(item, "predictedLiveState").or_else(|| state(item, "targetState"));
    let modified = item["modified"].as_bool() == Some(true);
    if !modified && live.is_some() == desired.is_some() {
        return None;
    }
    let text = |key: &str| item[key].as_str().unwrap_or_default().to_string();
    Some(Drifted {
        group: text("group"),
        kind: text("kind"),
        namespace: Some(text("namespace")).filter(|one| !one.is_empty()),
        name: text("name"),
        live,
        desired,
    })
}

/// A state Argo CD sends as JSON inside a string; none for `null` or empty.
fn state(item: &Value, key: &str) -> Option<Value> {
    let text = item[key].as_str()?;
    let value: Value = serde_json::from_str(text).ok()?;
    (!value.is_null()).then_some(value)
}

fn first_line(body: &str) -> String {
    body.lines().next().unwrap_or_default().to_string()
}
