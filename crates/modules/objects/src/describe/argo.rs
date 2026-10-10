//! An Argo CD Application's own part: its sources, destination, sync policy, last sync and conditions.

use groove_types::{AppCondition, AppPart, AppSource, Destination, Operation, SyncedResource};
use serde_json::Value;

use super::{text, time};

/// Whether `object` is Argo CD's, not another group's kind of the same name.
pub(super) fn is_application(kind: &str, object: &Value) -> bool {
    let group = object["apiVersion"].as_str().unwrap_or_default();
    kind == "Application" && group.starts_with("argoproj.io/")
}

pub(super) fn part(app: &Value) -> AppPart {
    let (spec, status) = (&app["spec"], &app["status"]);
    let automated = spec.pointer("/syncPolicy/automated").map(|on| {
        let set = |key: &str| on[key].as_bool().unwrap_or_default();
        (set("prune"), set("selfHeal"))
    });
    AppPart {
        project: text(spec, "/project"),
        sync: text(status, "/sync/status"),
        health: text(status, "/health/status"),
        sources: sources(spec, status),
        synced: Some(crate::argo::revisions(Some(status))).filter(|one| !one.is_empty()),
        destination: Destination {
            server: optional(&spec["destination"], "/server"),
            name: optional(&spec["destination"], "/name"),
            namespace: optional(&spec["destination"], "/namespace"),
        },
        automated,
        operation: operation(&status["operationState"]),
        conditions: conditions(&status["conditions"]),
    }
}

/// Its one source, or each of its several, with the revision each synced to, in the same order.
fn sources(spec: &Value, status: &Value) -> Vec<AppSource> {
    let several = spec["sources"]
        .as_array()
        .map(|all| all.iter().collect::<Vec<_>>());
    let all = several.unwrap_or_else(|| vec![&spec["source"]]);
    let all = all.into_iter().filter(|one| one.is_object());
    let synced = |at: usize| match status.pointer("/sync/revisions") {
        Some(list) => list[at].as_str().map(crate::argo::short),
        None => optional(status, "/sync/revision").map(|one| crate::argo::short(&one)),
    };
    all.enumerate()
        .map(|(at, one)| AppSource {
            repo: text(one, "/repoURL"),
            path: optional(one, "/path"),
            chart: optional(one, "/chart"),
            target: text(one, "/targetRevision"),
            values: strings(one.pointer("/helm/valueFiles")),
            reference: optional(one, "/ref"),
            synced: synced(at),
        })
        .collect()
}

fn operation(state: &Value) -> Option<Operation> {
    let phase = state["phase"].as_str()?.to_string();
    let result = &state["syncResult"];
    let by = match state
        .pointer("/operation/initiatedBy/automated")
        .and_then(Value::as_bool)
    {
        Some(true) => "auto-sync".to_string(),
        _ => text(state, "/operation/initiatedBy/username"),
    };
    let listed = result["resources"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default();
    let failed = listed
        .iter()
        .filter(|one| broke(one))
        .map(resource)
        .collect();
    Some(Operation {
        phase,
        message: text(state, "/message"),
        revision: Some(crate::argo::revisions(Some(
            &serde_json::json!({ "sync": result }),
        )))
        .filter(|one| !one.is_empty()),
        started: time(state, "/startedAt"),
        finished: time(state, "/finishedAt"),
        retries: state["retryCount"].as_u64().unwrap_or_default() as u32,
        by,
        failed,
    })
}

/// A result that did not apply, or a hook that did not succeed.
fn broke(one: &Value) -> bool {
    let status = one["status"].as_str().unwrap_or_default();
    let hook = one["hookPhase"].as_str().unwrap_or_default();
    matches!(status, "SyncFailed") || matches!(hook, "Failed" | "Error")
}

fn resource(one: &Value) -> SyncedResource {
    SyncedResource {
        kind: text(one, "/kind"),
        name: text(one, "/name"),
        namespace: optional(one, "/namespace").filter(|one| !one.is_empty()),
        status: text(one, "/status"),
        message: text(one, "/message"),
        hook: !text(one, "/hookType").is_empty(),
    }
}

fn conditions(all: &Value) -> Vec<AppCondition> {
    let all = all.as_array().map(Vec::as_slice).unwrap_or_default();
    all.iter()
        .map(|one| AppCondition {
            kind: text(one, "/type"),
            message: text(one, "/message"),
            at: time(one, "/lastTransitionTime"),
        })
        .collect()
}

fn optional(value: &Value, at: &str) -> Option<String> {
    value.pointer(at).and_then(Value::as_str).map(String::from)
}

fn strings(list: Option<&Value>) -> Vec<String> {
    let all = list
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    all.iter()
        .filter_map(|one| one.as_str().map(String::from))
        .collect()
}
