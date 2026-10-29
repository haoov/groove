//! The kinds a cluster serves, from aggregated discovery or, where it reads empty, a group at a time.

use kube::client::APIGroupDiscovery;
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
    #[serde(default)]
    pub short_names: Vec<String>,
}

impl Client {
    pub async fn kinds(&self) -> Result<Vec<Kind>> {
        self.answered(self.discovered()).await
    }

    async fn discovered(&self) -> Result<Vec<Kind>> {
        if let Some(kinds) = self.aggregated().await.filter(|kinds| !kinds.is_empty()) {
            return Ok(kinds);
        }
        let found = Discovery::new(self.inner.clone())
            .run()
            .await
            .map_err(|e| Error::of(&self.context, e))?;
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
                    short_names: Vec::new(),
                });
            }
        }
        Ok(kinds)
    }

    /// The kinds of aggregated discovery, short names included; `None` where it is not served.
    async fn aggregated(&self) -> Option<Vec<Kind>> {
        let core = self.inner.list_core_api_versions_aggregated().await.ok()?;
        let apis = self.inner.list_api_groups_aggregated().await.ok()?;
        let groups = core.items.into_iter().chain(apis.items);
        Some(groups.flat_map(preferred).collect())
    }
}

/// A group's kinds at its preferred version, the first one served.
fn preferred(group: APIGroupDiscovery) -> Vec<Kind> {
    let name = group.metadata.and_then(|one| one.name).unwrap_or_default();
    let Some(version) = group.versions.into_iter().next() else {
        return Vec::new();
    };
    let at = version.version.unwrap_or_default();
    let kinds = version.resources.into_iter().filter_map(|resource| {
        let kind = resource.response_kind?.kind?;
        let can = |verb: &str| resource.verbs.iter().any(|one| one == verb);
        Some(Kind {
            group: name.clone(),
            version: at.clone(),
            kind,
            plural: resource.resource.clone()?,
            namespaced: resource.scope.as_deref() == Some("Namespaced"),
            watchable: can("list") && can("watch"),
            short_names: resource.short_names.clone(),
        })
    });
    kinds.collect()
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
