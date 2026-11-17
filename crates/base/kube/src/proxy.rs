//! A service inside the cluster, reached through the API server's service proxy.

use futures_util::Stream;
use k8s_openapi::api::core::v1::{ConfigMap, Service};
use kube::Api;
use kube::api::ListParams;

use crate::{Client, Error, Result};

/// A service by namespace, name and the port it is reached on, with its scheme.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proxied {
    pub namespace: String,
    pub name: String,
    pub scheme: &'static str,
    pub port: i32,
}

/// A service found: where it stands, and its ports by name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    pub namespace: String,
    pub name: String,
    pub ports: Vec<(String, i32)>,
}

fn found(service: Service) -> Found {
    let ports = service.spec.and_then(|spec| spec.ports).unwrap_or_default();
    Found {
        namespace: service.metadata.namespace.unwrap_or_default(),
        name: service.metadata.name.unwrap_or_default(),
        ports: ports
            .into_iter()
            .map(|one| (one.name.unwrap_or_default(), one.port))
            .collect(),
    }
}

impl Proxied {
    fn uri(&self, path: &str) -> String {
        let (namespace, name) = (&self.namespace, &self.name);
        let (scheme, port) = (self.scheme, self.port);
        format!("/api/v1/namespaces/{namespace}/services/{scheme}:{name}:{port}/proxy{path}")
    }
}

impl Client {
    /// The first service in any namespace whose labels match `selector`.
    pub async fn service(&self, selector: &str) -> Result<Option<Found>> {
        let api: Api<Service> = Api::all(self.inner.clone());
        let params = ListParams::default().labels(selector);
        let listed = self
            .answered(async {
                let asked = api.list(&params).await;
                asked.map_err(|e| Error::of(&self.context, e))
            })
            .await?;
        Ok(listed.items.into_iter().next().map(found))
    }

    /// One key of a ConfigMap; none when the map or the key is missing.
    pub async fn config_value(
        &self,
        namespace: &str,
        name: &str,
        key: &str,
    ) -> Result<Option<String>> {
        let api: Api<ConfigMap> = Api::namespaced(self.inner.clone(), namespace);
        let found = self
            .answered(async {
                let asked = api.get_opt(name).await;
                asked.map_err(|e| Error::of(&self.context, e))
            })
            .await?;
        Ok(found.and_then(|map| map.data?.remove(key)))
    }

    /// The body `path` answers on the service, with `headers` added.
    pub async fn proxy_get(
        &self,
        to: &Proxied,
        path: &str,
        headers: &[(&str, &str)],
    ) -> Result<String> {
        let request = request(to, path, headers)?;
        self.answered(async {
            let asked = self.inner.request_text(request).await;
            asked.map_err(|e| Error::proxied(&self.context, e))
        })
        .await
    }

    /// The lines `path` streams from the service as they come, with `headers` added.
    pub async fn proxy_lines(
        &self,
        to: &Proxied,
        path: &str,
        headers: &[(&str, &str)],
    ) -> Result<impl Stream<Item = Result<String>> + use<>> {
        self.lines(request(to, path, headers)?, None).await
    }
}

fn request(to: &Proxied, path: &str, headers: &[(&str, &str)]) -> Result<http::Request<Vec<u8>>> {
    let mut asked = http::Request::get(to.uri(path));
    for (name, value) in headers {
        asked = asked.header(*name, *value);
    }
    asked
        .body(Vec::new())
        .map_err(|e| Error::Kubeconfig(e.to_string()))
}
