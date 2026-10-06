//! The kinds a cluster serves, from aggregated discovery or, where it reads empty, a group at a time.

use kube::discovery::{Discovery, Scope};

use crate::{Client, Error, Result};

/// One kind the cluster serves at its preferred version.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Kind {
    /// Empty for the core group.
    pub group: String,
    pub version: String,
    pub kind: String,
    pub plural: String,
    pub namespaced: bool,
    /// The kind can be listed and watched.
    pub watchable: bool,
}

impl Client {
    pub async fn kinds(&self) -> Result<Vec<Kind>> {
        let fresh = || Discovery::new(self.inner.clone());
        let aggregated = fresh().run_aggregated().await.ok();
        let found = match aggregated.filter(|found| found.groups().next().is_some()) {
            Some(found) => found,
            None => fresh()
                .run()
                .await
                .map_err(|e| Error::of(&self.context, e))?,
        };
        let mut kinds = Vec::new();
        for group in found.groups() {
            for (resource, caps) in group.recommended_resources() {
                let can = |verb: &str| caps.operations.iter().any(|one| one == verb);
                kinds.push(Kind {
                    group: resource.group,
                    version: resource.version,
                    kind: resource.kind,
                    plural: resource.plural,
                    namespaced: caps.scope == Scope::Namespaced,
                    watchable: can("list") && can("watch"),
                });
            }
        }
        Ok(kinds)
    }
}

impl Kind {
    /// The path its objects are listed at: in one namespace, or across all of them.
    pub(crate) fn path(&self, namespace: Option<&str>) -> String {
        let base = match self.group.is_empty() {
            true => format!("/api/{}", self.version),
            false => format!("/apis/{}/{}", self.group, self.version),
        };
        match namespace.filter(|_| self.namespaced) {
            Some(namespace) => format!("{base}/namespaces/{namespace}/{}", self.plural),
            None => format!("{base}/{}", self.plural),
        }
    }
}
