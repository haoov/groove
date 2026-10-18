//! Writes to one object: a merge patch, tried on the server or stored.

use std::path::PathBuf;

use groove_kube::{Client, Patch};
use groove_types::{Error, ErrorKind, Result, Target};
use serde_json::Value;

use crate::discovery::failed;

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
    client.patch_object(patch).await.map_err(|e| match e {
        groove_kube::Error::Conflict { .. } => Error::new(
            ErrorKind::Conflict,
            format!("{} changed since it was read", target.label()),
        ),
        other => failed(other),
    })
}

fn path(target: &Target) -> String {
    let kind = crate::kube_kind(&target.kind);
    let mut path = format!("{}/{}", kind.path(target.namespace.as_deref()), target.name);
    if let Some(subresource) = &target.subresource {
        path = format!("{path}/{subresource}");
    }
    path
}
