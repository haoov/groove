//! The kubeconfig files, merged as kubectl merges them: the first file to name a thing wins.

use std::path::PathBuf;

use kube::config::{AuthInfo, Kubeconfig};

use crate::{Error, Result};

/// One context a kubeconfig names. The credentials stay in the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Context {
    pub name: String,
    pub cluster: String,
    pub server: Option<String>,
    pub namespace: Option<String>,
    pub auth: Auth,
}

/// How a context signs in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Auth {
    /// A plugin run for each token, `kubelogin` and its like.
    Exec(String),
    Certificate,
    Token,
    None,
}

/// Every context the files name, in the order they name them. A missing file is skipped.
pub async fn contexts(paths: &[PathBuf]) -> Result<Vec<Context>> {
    let merged = merged(paths.to_vec()).await?;
    Ok(merged.contexts.iter().map(|one| of(&merged, one)).collect())
}

/// The files read and merged off the main thread, as one kubeconfig.
pub(crate) async fn merged(paths: Vec<PathBuf>) -> Result<Kubeconfig> {
    let read = tokio::task::spawn_blocking(move || {
        let mut merged = Kubeconfig::default();
        for path in paths.iter().filter(|path| path.exists()) {
            merged = merged.merge(Kubeconfig::read_from(path)?)?;
        }
        Ok::<_, kube::config::KubeconfigError>(merged)
    });
    match read.await {
        Ok(read) => read.map_err(|e| Error::Kubeconfig(e.to_string())),
        Err(e) => Err(Error::Kubeconfig(e.to_string())),
    }
}

fn of(kubeconfig: &Kubeconfig, named: &kube::config::NamedContext) -> Context {
    let context = named.context.clone().unwrap_or_default();
    let server = kubeconfig
        .clusters
        .iter()
        .find(|one| one.name == context.cluster)
        .and_then(|one| one.cluster.as_ref())
        .and_then(|cluster| cluster.server.clone());
    let user = context.user.as_ref().and_then(|user| {
        let named = kubeconfig.auth_infos.iter().find(|one| &one.name == user);
        named.and_then(|one| one.auth_info.as_ref())
    });
    Context {
        name: named.name.clone(),
        cluster: context.cluster,
        server,
        namespace: context.namespace,
        auth: user.map_or(Auth::None, auth),
    }
}

fn auth(user: &AuthInfo) -> Auth {
    if let Some(command) = user.exec.as_ref().and_then(|exec| exec.command.clone()) {
        return Auth::Exec(command);
    }
    if user.client_certificate.is_some() || user.client_certificate_data.is_some() {
        return Auth::Certificate;
    }
    if user.token.is_some() || user.token_file.is_some() {
        return Auth::Token;
    }
    Auth::None
}
