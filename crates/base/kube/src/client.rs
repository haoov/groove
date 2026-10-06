use std::path::PathBuf;

use kube::config::{Config, KubeConfigOptions};

use crate::kubeconfig::merged;
use crate::{Error, Result};

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
        let kubeconfig = merged(paths.to_vec()).await?;
        if !kubeconfig.contexts.iter().any(|one| one.name == context) {
            return Err(Error::NoContext(context.to_string()));
        }
        let options = KubeConfigOptions {
            context: Some(context.to_string()),
            ..KubeConfigOptions::default()
        };
        let config = Config::from_custom_kubeconfig(kubeconfig, &options)
            .await
            .map_err(|e| Error::Kubeconfig(e.to_string()))?;
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
            .inner
            .apiserver_version()
            .await
            .map_err(|e| Error::of(&self.context, e))?;
        Ok(info.git_version)
    }
}
