//! What differs between an Application's desired and live state, as YAML file pairs for the diff.

use std::path::PathBuf;

use serde_json::Value;

use crate::argo_tree::{ArgoApp, Opened};

/// One resource that differs: its path in the diff, its live YAML and its desired YAML.
pub type DriftFile = (String, String, String);

/// `app`'s drift read once, hooks left out; a side missing reads as empty.
pub async fn argo_drift_once(
    (paths, home): (Vec<PathBuf>, PathBuf),
    app: ArgoApp,
) -> Result<Vec<DriftFile>, String> {
    let opened = Opened::of((&paths, &home), &app).await?;
    let body = opened
        .get(&groove_argocd::drift_path(&app.namespace, &app.name))
        .await?;
    let drifted = groove_argocd::drift(&body).map_err(|why| opened.refused(&why))?;
    Ok(drifted.into_iter().map(file).collect())
}

fn file(one: groove_argocd::Drifted) -> DriftFile {
    let namespace = one.namespace.as_deref().unwrap_or("cluster");
    let path = format!("argo/{}/{namespace}/{}.yaml", one.kind, one.name);
    let side = |state: Option<Value>| {
        state
            .map(|one| crate::yaml::yaml(&shown(one)))
            .unwrap_or_default()
    };
    (path, side(one.live), side(one.desired))
}

/// The object as the diff shows it: no status, and no field the server alone writes.
fn shown(mut object: Value) -> Value {
    if let Some(all) = object.as_object_mut() {
        all.remove("status");
    }
    if let Some(metadata) = object.get_mut("metadata").and_then(Value::as_object_mut) {
        for key in [
            "managedFields",
            "resourceVersion",
            "generation",
            "uid",
            "creationTimestamp",
        ] {
            metadata.remove(key);
        }
    }
    object
}
