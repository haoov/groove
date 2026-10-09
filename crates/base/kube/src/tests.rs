mod client;
mod kubeconfig;
mod objects;
mod table;

use std::path::{Path, PathBuf};

pub(crate) fn file(dir: &Path, name: &str, yaml: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, yaml).expect("a kubeconfig written");
    path
}

/// One context named `name`, on `server`, signing in with a fixed token.
pub(crate) fn token_context(name: &str, server: &str) -> String {
    format!(
        "apiVersion: v1
kind: Config
clusters:
- name: {name}
  cluster:
    server: {server}
users:
- name: {name}
  user:
    token: not-a-real-token
contexts:
- name: {name}
  context:
    cluster: {name}
    user: {name}
    namespace: default
"
    )
}
