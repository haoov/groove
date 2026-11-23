//! One object deleted, tried on the server or done.

use http::header::{ACCEPT, CONTENT_TYPE};
use serde_json::{Value, json};

use crate::{Client, Error, Result};

/// A delete of the object at `path`.
#[derive(Debug, Clone, Copy)]
pub struct Delete<'a> {
    /// `/apis/apps/v1/namespaces/paxone/deployments/api`.
    pub path: &'a str,
    /// The `resourceVersion` the delete was asked on: the server refuses it once the object moved.
    pub version: Option<&'a str>,
    /// The server checks the delete, but removes nothing.
    pub dry_run: bool,
}

impl Delete<'_> {
    fn uri(&self) -> String {
        match self.dry_run {
            true => format!("{}?dryRun=All", self.path),
            false => self.path.to_string(),
        }
    }

    /// Its dependents deleted after it, as `kubectl delete` does.
    fn body(&self) -> Value {
        let mut options = json!({ "kind": "DeleteOptions", "apiVersion": "v1",
            "propagationPolicy": "Background" });
        if let Some(version) = self.version {
            options["preconditions"] = json!({ "resourceVersion": version });
        }
        if self.dry_run {
            // The server reads a delete's options from its body and drops the query's.
            options["dryRun"] = json!(["All"]);
        }
        options
    }
}

impl Client {
    /// Deleted, or only checked on a dry run; a moved object is `Conflict`.
    pub async fn delete_object(&self, delete: Delete<'_>) -> Result<()> {
        let body = serde_json::to_vec(&delete.body()).map_err(|e| Error::Unreadable {
            context: self.context.clone(),
            detail: e.to_string(),
        })?;
        let request = http::Request::delete(delete.uri())
            .header(ACCEPT, "application/json")
            .header(CONTENT_TYPE, "application/json")
            .body(body)
            .map_err(|e| Error::Kubeconfig(e.to_string()))?;
        let _: Value = self
            .answered(async {
                let asked = self.inner.request(request).await;
                asked.map_err(|e| Error::of(&self.context, e))
            })
            .await?;
        Ok(())
    }
}
