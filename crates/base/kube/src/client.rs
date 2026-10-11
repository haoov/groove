use std::future::Future;
use std::path::PathBuf;
use std::time::Duration;

use http::HeaderValue;
use http::header::USER_AGENT;
use kube::config::{Config, KubeConfigOptions};

use crate::kubeconfig::merged;
use crate::{Error, Result};

/// What the API server's metrics and audit log name Groove's requests by.
pub const AGENT: &str = concat!("groove/", env!("CARGO_PKG_VERSION"));

/// How long a request, the sign-in it runs included, waits for its answer.
pub(crate) const ANSWER: Duration = Duration::from_secs(60);

/// A connection to the cluster of one context.

#[derive(Clone)]
pub struct Client {
    pub(crate) inner: kube::Client,
    pub(crate) context: String,
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("context", &self.context)
            .finish()
    }
}

impl Client {
    /// A client for `context`, its credentials read from the files as they stand now.
    pub async fn connect(paths: &[PathBuf], context: &str) -> Result<Self> {
        let made = tokio::time::timeout(ANSWER, Self::made(paths, context)).await;
        made.unwrap_or_else(|_| Err(Error::silent(context, ANSWER)))
    }

    async fn made(paths: &[PathBuf], context: &str) -> Result<Self> {
        let kubeconfig = merged(paths.to_vec()).await?;
        if !kubeconfig.contexts.iter().any(|one| one.name == context) {
            return Err(Error::NoContext(context.to_string()));
        }
        let options = KubeConfigOptions {
            context: Some(context.to_string()),
            ..KubeConfigOptions::default()
        };
        let mut config = Config::from_custom_kubeconfig(kubeconfig, &options)
            .await
            .map_err(|e| Error::Kubeconfig(e.to_string()))?;
        config
            .headers
            .push((USER_AGENT, HeaderValue::from_static(AGENT)));
        let inner = kube::Client::try_from(config).map_err(|e| Error::of(context, e))?;
        Ok(Self {
            inner,
            context: context.to_string(),
        })
    }

    pub fn context(&self) -> &str {
        &self.context
    }

    /// The server's version: the cheapest call that needs a sign-in.
    pub async fn version(&self) -> Result<String> {
        let info = self
            .answered(async {
                let asked = self.inner.apiserver_version().await;
                asked.map_err(|e| Error::of(&self.context, e))
            })
            .await?;
        Ok(info.git_version)
    }

    pub(crate) async fn answered<T>(&self, asked: impl Future<Output = Result<T>>) -> Result<T> {
        let within = tokio::time::timeout(ANSWER, asked).await;
        within.unwrap_or_else(|_| Err(Error::silent(&self.context, ANSWER)))
    }
}
