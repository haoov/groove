//! Reads made once, not watched: a pod's usage from metrics-server, a Helm release's revision.

use std::path::PathBuf;

use groove_kube::{Client, Kind, Narrowed};
use groove_types::{Result, Usage, quantity};
use serde_json::Value;

use crate::discovery::failed;

/// What each container of the pod uses now; none where the cluster runs no metrics-server.
pub async fn usage(
    paths: &[PathBuf],
    context: &str,
    namespace: &str,
    pod: &str,
) -> Result<Option<Usage>> {
    let client = Client::connect(paths, context).await.map_err(failed)?;
    let path = format!("/apis/metrics.k8s.io/v1beta1/namespaces/{namespace}/pods/{pod}");
    let read = match client.get_object(&path).await {
        Ok(read) => read,
        Err(groove_kube::Error::Api {
            status: 404 | 503, ..
        }) => return Ok(None),
        Err(e) => return Err(failed(e)),
    };
    let containers = read["containers"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default();
    let amount = |one: &Value, at: &str| one.pointer(at).and_then(Value::as_str).and_then(quantity);
    let containers = containers
        .iter()
        .map(|one| {
            let name = one["name"].as_str().unwrap_or_default().to_string();
            let cpu = amount(one, "/usage/cpu").unwrap_or_default();
            (name, cpu, amount(one, "/usage/memory").unwrap_or_default())
        })
        .collect();
    Ok(Some(Usage { containers }))
}

/// The latest revision of the Helm release `release`, from its secrets' labels alone.
pub async fn helm_revision(
    paths: &[PathBuf],
    context: &str,
    namespace: &str,
    release: &str,
) -> Result<Option<u32>> {
    let client = Client::connect(paths, context).await.map_err(failed)?;
    let secrets = Kind {
        group: String::new(),
        version: "v1".into(),
        kind: "Secret".into(),
        plural: "secrets".into(),
        namespaced: true,
        watchable: true,
    };
    let labels = format!("owner=helm,name={release}");
    let narrowed = Narrowed {
        kind: &secrets,
        namespace: Some(namespace),
        fields: None,
        labels: Some(&labels),
    };
    let found = client.list_metadata(narrowed).await.map_err(failed)?;
    let version = |one: &Value| {
        let label = one
            .pointer("/metadata/labels/version")
            .and_then(Value::as_str)?;
        label.parse::<u32>().ok()
    };
    Ok(found.iter().filter_map(version).max())
}
