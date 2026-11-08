//! Writes to one object: a merge patch, tried on the server or stored.

use std::path::PathBuf;

use groove_kube::{Client, Patch};
use groove_types::{Error, ErrorKind, Result, Target};
use serde_json::Value;

use crate::discovery::failed;

/// A patch tried on the server: the version it was tried on, and what it changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preview {
    pub version: String,
    /// The object's YAML as it stands, and as the patch leaves it.
    pub before: String,
    pub after: String,
    /// A unified diff of the two; empty when nothing changes.
    pub diff: String,
}

/// The object as `body` leaves it; a version the object moved past is a `Conflict`.
pub async fn patch(
    paths: &[PathBuf],
    target: &Target,
    body: &Value,
    version: Option<&str>,
    dry_run: bool,
) -> Result<Value> {
    let client = Client::shared(paths, &target.context)
        .await
        .map_err(failed)?;
    let path = path(target);
    let patch = Patch {
        path: &path,
        body,
        version,
        dry_run,
    };
    client
        .patch_object(patch)
        .await
        .map_err(|e| moved(target, e))
}

/// The object read now, then `body` tried on that version and nothing stored.
pub async fn preview(paths: &[PathBuf], target: &Target, body: &Value) -> Result<Preview> {
    let client = Client::shared(paths, &target.context)
        .await
        .map_err(failed)?;
    let live = client.get_object(&path(target)).await.map_err(failed)?;
    let version = live
        .pointer("/metadata/resourceVersion")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let tried = patch(paths, target, body, Some(&version), true).await?;
    let (before, after) = crate::changed::sides(&live, &tried);
    let diff = crate::changed::unified(&before, &after);
    Ok(Preview {
        version,
        before,
        after,
        diff,
    })
}

fn moved(target: &Target, error: groove_kube::Error) -> Error {
    match error {
        groove_kube::Error::Conflict { .. } => Error::new(
            ErrorKind::Conflict,
            format!("{} changed since it was read", target.label()),
        ),
        other => failed(other),
    }
}

fn path(target: &Target) -> String {
    let kind = crate::kube_kind(&target.kind);
    let mut path = format!("{}/{}", kind.path(target.namespace.as_deref()), target.name);
    if let Some(subresource) = &target.subresource {
        path = format!("{path}/{subresource}");
    }
    path
}
