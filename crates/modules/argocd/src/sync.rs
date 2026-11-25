//! A sync and a refresh asked of Argo CD for one Application.

use serde_json::{Value, json};

/// One resource a sync is limited to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resource {
    pub group: String,
    pub kind: String,
    pub namespace: Option<String>,
    pub name: String,
}

/// Where a sync of the Application `name` is asked.
pub fn sync_path(name: &str) -> String {
    format!("/api/v1/applications/{name}/sync")
}

/// What a sync asks, with no `syncOptions`: Argo CD then applies the Application's own.
pub fn sync_body(namespace: &str, prune: bool, resources: &[Resource]) -> Vec<u8> {
    let mut body = json!({ "appNamespace": namespace, "prune": prune });
    if !resources.is_empty() {
        let each = resources.iter().map(|one| {
            json!({ "group": one.group, "kind": one.kind,
                "namespace": one.namespace.clone().unwrap_or_default(), "name": one.name })
        });
        body["resources"] = Value::Array(each.collect());
    }
    body.to_string().into_bytes()
}

/// The Application `namespace/name` read again from its sources; `hard` skips the manifest cache.
pub fn refresh_path(namespace: &str, name: &str, hard: bool) -> String {
    let how = if hard { "hard" } else { "normal" };
    format!("/api/v1/applications/{name}?appNamespace={namespace}&refresh={how}")
}

/// Whether Argo CD took the ask: it answers with the Application, else with why not.
pub fn taken(body: &str) -> Result<(), String> {
    let json: Value = serde_json::from_str(body).map_err(|_| first_line(body))?;
    if json["metadata"].is_object() {
        return Ok(());
    }
    let said = json["message"].as_str().map(String::from);
    Err(said.unwrap_or_else(|| first_line(body)))
}

fn first_line(body: &str) -> String {
    body.lines().next().unwrap_or_default().to_string()
}
