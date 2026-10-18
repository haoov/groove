//! One object changed by a JSON merge patch, tried on the server or written.

use http::header::{ACCEPT, CONTENT_TYPE};
use serde_json::Value;

use crate::objects::bare;
use crate::{Client, Error, Result};

/// The name the server records as the owner of the fields a patch sets.
pub(crate) const MANAGER: &str = "groove";

/// A merge patch on the object at `path`.
#[derive(Debug, Clone, Copy)]
pub struct Patch<'a> {
    /// `/apis/apps/v1/namespaces/paxone/deployments/api`.
    pub path: &'a str,
    pub body: &'a Value,
    /// The `resourceVersion` the change was made on: the server refuses it once the object moved.
    pub version: Option<&'a str>,
    /// The server runs the patch and returns the result, but stores nothing.
    pub dry_run: bool,
}

impl Patch<'_> {
    fn uri(&self) -> String {
        let mut query = vec![format!("fieldManager={MANAGER}")];
        if self.dry_run {
            query.push("dryRun=All".to_string());
        }
        format!("{}?{}", self.path, query.join("&"))
    }

    fn body(&self) -> Value {
        let mut body = self.body.clone();
        if let (Some(version), Some(object)) = (self.version, body.as_object_mut()) {
            let metadata = object
                .entry("metadata")
                .or_insert_with(|| Value::Object(Default::default()));
            if let Some(metadata) = metadata.as_object_mut() {
                metadata.insert("resourceVersion".into(), Value::String(version.into()));
            }
        }
        body
    }
}

impl Client {
    /// The object as the patch leaves it, without `managedFields`; a moved object is `Conflict`.
    pub async fn patch_object(&self, patch: Patch<'_>) -> Result<Value> {
        let body = serde_json::to_vec(&patch.body()).map_err(|e| Error::Unreadable {
            context: self.context.clone(),
            detail: e.to_string(),
        })?;
        let request = http::Request::patch(patch.uri())
            .header(ACCEPT, "application/json")
            .header(CONTENT_TYPE, "application/merge-patch+json")
            .body(body)
            .map_err(|e| Error::Kubeconfig(e.to_string()))?;
        let object: Value = self
            .answered(async {
                let asked = self.inner.request(request).await;
                asked.map_err(|e| Error::of(&self.context, e))
            })
            .await?;
        Ok(bare(object))
    }
}
