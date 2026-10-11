//! Argo CD's Applications, listed with columns of their own: what they sync, and how.

use groove_types::{KubeKind, ObjectRow, TableColumn, Timestamp};

const COLUMNS: [&str; 6] = [
    "Name",
    "Project",
    "Sync Status",
    "Health Status",
    "Auto-sync",
    "Revision",
];

pub(crate) fn applications(kind: &KubeKind) -> bool {
    kind.group == "argoproj.io" && kind.kind == "Application"
}

/// The columns the list shows: the server's for a kind, Groove's own for Applications.
pub(crate) fn columns(kind: &KubeKind, server: &[TableColumn]) -> Vec<TableColumn> {
    if !applications(kind) {
        return server.to_vec();
    }
    let column = |name: &&str| TableColumn {
        name: name.to_string(),
        priority: 0,
        date: false,
    };
    COLUMNS.iter().map(column).collect()
}

/// The row as it arrived, its cells in the list's columns.
pub(crate) fn row(
    kind: &KubeKind,
    row: groove_kube::Row,
    server: &[TableColumn],
    received: Timestamp,
) -> ObjectRow {
    if !applications(kind) {
        return crate::row_of(row, server, received);
    }
    let by = |name: &str| {
        let at = server.iter().position(|one| one.name == name);
        at.and_then(|at| row.cells.get(at))
            .cloned()
            .unwrap_or_default()
    };
    let cells = vec![
        by("Name"),
        by("Project"),
        by("Sync Status"),
        by("Health Status"),
        automated(row.object.as_ref().map(|one| &one["spec"])),
        revisions(row.object.as_ref().map(|one| &one["status"])),
    ];
    ObjectRow {
        aging: Vec::new(),
        uid: row.uid,
        name: row.name,
        namespace: row.namespace,
        version: row.version,
        cells,
    }
}

/// `auto · prune · heal`, as its sync policy reads; `manual` without one.
pub(crate) fn automated(spec: Option<&serde_json::Value>) -> String {
    let Some(policy) = spec.and_then(|one| one.pointer("/syncPolicy/automated")) else {
        return "manual".to_string();
    };
    let on = |key: &str| policy[key].as_bool().unwrap_or_default();
    let mut said = "auto".to_string();
    if on("prune") {
        said.push_str(" · prune");
    }
    if on("selfHeal") {
        said.push_str(" · heal");
    }
    said
}

/// What it synced to: its one revision, or each of its sources', shortened.
pub(crate) fn revisions(status: Option<&serde_json::Value>) -> String {
    let Some(sync) = status.map(|one| &one["sync"]) else {
        return String::new();
    };
    if let Some(one) = sync["revision"].as_str().filter(|one| !one.is_empty()) {
        return short(one);
    }
    let all = sync["revisions"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default();
    let all: Vec<String> = all
        .iter()
        .filter_map(|one| one.as_str())
        .map(short)
        .collect();
    all.join(" · ")
}

/// A commit's first seven characters; a tag or a chart version as it is.
pub(crate) fn short(revision: &str) -> String {
    let commit = revision.len() == 40 && revision.chars().all(|one| one.is_ascii_hexdigit());
    match commit {
        true => revision[..7].to_string(),
        false => revision.to_string(),
    }
}
