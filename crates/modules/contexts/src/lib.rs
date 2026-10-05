//! The kubeconfig contexts on this machine, and whether each one signs in.

#[cfg(test)]
mod tests;

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use groove_kube::{Auth, Client, Context};
use groove_types::{Error, KubeAuth, KubeContext, Login, Result};

/// The files kubectl reads: `$KUBECONFIG` when it is set, else `~/.kube/config`.
pub fn paths(kubeconfig: Option<&OsStr>, home: &Path) -> Vec<PathBuf> {
    match kubeconfig.filter(|value| !value.is_empty()) {
        Some(value) => std::env::split_paths(value).collect(),
        None => vec![home.join(".kube/config")],
    }
}

/// Every context the files name, in their order.
pub async fn found(paths: &[PathBuf]) -> Result<Vec<KubeContext>> {
    let contexts = groove_kube::contexts(paths)
        .await
        .map_err(|e| Error::invalid(e.to_string()))?;
    Ok(contexts.into_iter().map(of).collect())
}

/// Whether `context` signs in. A refusal reads the files again and runs the plugin once more.
pub async fn check(paths: &[PathBuf], context: &str) -> Login {
    match version(paths, context).await {
        Err(groove_kube::Error::Refused { .. }) => login(version(paths, context).await),
        once => login(once),
    }
}

async fn version(paths: &[PathBuf], context: &str) -> groove_kube::Result<String> {
    Client::connect(paths, context).await?.version().await
}

fn login(answer: groove_kube::Result<String>) -> Login {
    match answer {
        Ok(version) => Login::SignedIn { version },
        Err(e @ groove_kube::Error::Refused { .. }) => Login::Refused(e.to_string()),
        Err(e @ groove_kube::Error::Unreachable { .. }) => Login::Unreachable(e.to_string()),
        Err(e) => Login::Failed(e.to_string()),
    }
}

fn of(context: Context) -> KubeContext {
    KubeContext {
        name: context.name,
        cluster: context.cluster,
        server: context.server,
        namespace: context.namespace,
        auth: match context.auth {
            Auth::Exec(command) => KubeAuth::Exec { command },
            Auth::Certificate => KubeAuth::Certificate,
            Auth::Token => KubeAuth::Token,
            Auth::None => KubeAuth::None,
        },
    }
}
