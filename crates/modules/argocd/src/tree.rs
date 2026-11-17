//! An Application's resource tree, as Argo CD's API answers or streams it, read as each resource's health.

use groove_types::ResourceHealth;
use serde_json::Value;

/// The label argocd-server's Service carries.
pub const SERVER_LABEL: &str = "app.kubernetes.io/name=argocd-server";

/// The tree of the Application `namespace/name`, read once or `streamed`.
pub fn tree_path(namespace: &str, name: &str, streamed: bool) -> String {
    let stream = if streamed { "/stream" } else { "" };
    format!("/api/v1{stream}/applications/{name}/resource-tree?appNamespace={namespace}")
}

/// Each node's health in a tree; `Err` holds what Argo CD said when the body is no tree.
pub fn health(body: &str) -> Result<Vec<ResourceHealth>, String> {
    let json: Value = serde_json::from_str(body).map_err(|_| first_line(body))?;
    match json["nodes"].as_array() {
        Some(nodes) => Ok(nodes.iter().map(node).collect()),
        None => Err(said(&json).unwrap_or_else(|| first_line(body))),
    }
}

/// One line of the stream: a new tree, an error, or nothing to read.
pub fn streamed(line: &str) -> Option<Result<Vec<ResourceHealth>, String>> {
    let line = line.trim().trim_start_matches("data:").trim();
    let json: Value = serde_json::from_str(line).ok()?;
    if let Some(error) = json.get("error") {
        return Some(Err(said(error).unwrap_or_else(|| error.to_string())));
    }
    json.get("result").map(|tree| health(&tree.to_string()))
}

fn node(one: &Value) -> ResourceHealth {
    let text = |key: &str| one[key].as_str().unwrap_or_default().to_string();
    let namespace = Some(text("namespace")).filter(|one| !one.is_empty());
    ResourceHealth {
        group: text("group"),
        kind: text("kind"),
        namespace,
        name: text("name"),
        status: one["health"]["status"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        reason: one["health"]["message"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
    }
}

fn said(json: &Value) -> Option<String> {
    json["message"].as_str().map(String::from)
}

fn first_line(body: &str) -> String {
    body.lines().next().unwrap_or_default().to_string()
}
