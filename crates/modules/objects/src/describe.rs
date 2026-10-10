//! A whole object read into what its tab draws, off the main thread.

mod argo;
mod container;
mod pod;

use std::sync::Arc;

use groove_types::{Described, EventRow, Owner, Timestamp};
use serde_json::Value;

/// The object as its tab draws it; `kind` picks the part only some kinds have, `whole` its YAML.
pub fn describe(kind: &str, object: &Value, whole: bool) -> Described {
    let meta = &object["metadata"];
    let yaml = match whole {
        true => Arc::from(crate::yaml::yaml(object)),
        false => Arc::from(""),
    };
    Described {
        uid: text(meta, "/uid"),
        name: text(meta, "/name"),
        namespace: meta["namespace"].as_str().map(String::from),
        labels: pairs(&meta["labels"]),
        annotations: pairs(&meta["annotations"]),
        created: time(meta, "/creationTimestamp"),
        owners: owners(&meta["ownerReferences"]),
        replicas: replicas(kind, object),
        selector: match kind {
            "Service" => pairs(&object["spec"]["selector"]),
            _ => Vec::new(),
        },
        pod: (kind == "Pod").then(|| Box::new(pod::part(object))),
        app: argo::is_application(kind, object).then(|| Box::new(argo::part(object))),
        event: (kind == "Event").then(|| event(object)),
        yaml,
    }
}

fn owners(refs: &Value) -> Vec<Owner> {
    let refs = refs.as_array().map(Vec::as_slice).unwrap_or_default();
    refs.iter()
        .map(|one| Owner {
            api_version: text(one, "/apiVersion"),
            kind: text(one, "/kind"),
            name: text(one, "/name"),
        })
        .collect()
}

/// Ready and desired replicas of a workload.
fn replicas(kind: &str, object: &Value) -> Option<(i64, i64)> {
    let number = |at: &str, or: i64| object.pointer(at).and_then(Value::as_i64).unwrap_or(or);
    match kind {
        "Deployment" | "StatefulSet" | "ReplicaSet" => Some((
            number("/status/readyReplicas", 0),
            number("/spec/replicas", 1),
        )),
        "DaemonSet" => Some((
            number("/status/numberReady", 0),
            number("/status/desiredNumberScheduled", 0),
        )),
        _ => None,
    }
}

fn event(object: &Value) -> EventRow {
    let count = object["count"]
        .as_u64()
        .or_else(|| object.pointer("/series/count").and_then(Value::as_u64));
    let last = [
        "/lastTimestamp",
        "/series/lastObservedTime",
        "/eventTime",
        "/firstTimestamp",
    ]
    .iter()
    .find_map(|at| time(object, at))
    .or_else(|| time(object, "/metadata/creationTimestamp"));
    let about = format!(
        "{}/{}",
        text(object, "/involvedObject/kind").to_lowercase(),
        text(object, "/involvedObject/name")
    );
    EventRow {
        warning: object["type"] == "Warning",
        reason: text(object, "/reason"),
        count: count.unwrap_or(1) as u32,
        last,
        message: text(object, "/message").trim().to_string(),
        about,
    }
}

pub(crate) fn text(value: &Value, at: &str) -> String {
    value
        .pointer(at)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

pub(crate) fn time(value: &Value, at: &str) -> Option<Timestamp> {
    let raw = value.pointer(at).and_then(Value::as_str)?;
    Timestamp::parse(raw).ok()
}

/// A map of strings as its pairs, in key order.
pub(crate) fn pairs(map: &Value) -> Vec<(String, String)> {
    let Some(map) = map.as_object() else {
        return Vec::new();
    };
    let value = |one: &Value| one.as_str().map_or_else(|| one.to_string(), String::from);
    map.iter()
        .map(|(key, one)| (key.clone(), value(one)))
        .collect()
}
