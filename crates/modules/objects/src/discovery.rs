//! A context's kinds, read from the disk while the cache is fresh, else asked and kept; its namespaces.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use groove_kube::{Client, Kind};
use groove_types::{Error, KubeKind, Result};

/// How long a context's kinds are read from the disk before the cluster is asked again.
pub const FRESH: Duration = Duration::from_secs(10 * 60);

/// The kinds of `context`; `again` asks the cluster whatever the cache holds.
pub async fn kinds(
    paths: &[PathBuf],
    context: &str,
    cache: &Path,
    again: bool,
) -> Result<Vec<KubeKind>> {
    let file = cache.join(file_name(context));
    if !again && let Some(kinds) = cached(&file).await {
        return Ok(kinds.into_iter().map(crate::kind_of).collect());
    }
    let client = Client::connect(paths, context).await.map_err(failed)?;
    let kinds = client.kinds().await.map_err(failed)?;
    keep(&file, &kinds).await;
    Ok(kinds.into_iter().map(crate::kind_of).collect())
}

/// One file a context, its name safe for any filesystem.
fn file_name(context: &str) -> String {
    let safe: String = context
        .chars()
        .map(
            |c| match c.is_ascii_alphanumeric() || c == '-' || c == '.' {
                true => c,
                false => '_',
            },
        )
        .collect();
    format!("{safe}.json")
}

async fn cached(file: &Path) -> Option<Vec<Kind>> {
    let meta = tokio::fs::metadata(file).await.ok()?;
    let age = SystemTime::now()
        .duration_since(meta.modified().ok()?)
        .ok()?;
    if age > FRESH {
        return None;
    }
    let text = tokio::fs::read_to_string(file).await.ok()?;
    serde_json::from_str(&text).ok()
}

/// The cache is a convenience: a write that fails leaves the next read to ask the cluster.
async fn keep(file: &Path, kinds: &[Kind]) {
    let Ok(text) = serde_json::to_string(kinds) else {
        return;
    };
    if let Some(dir) = file.parent() {
        let _ = tokio::fs::create_dir_all(dir).await;
    }
    let _ = tokio::fs::write(file, text).await;
}

pub(crate) fn failed(error: groove_kube::Error) -> Error {
    Error::invalid(error.to_string())
}

/// The names of every namespace of `context`, every page of them.
pub async fn namespaces(paths: &[PathBuf], context: &str) -> Result<Vec<String>> {
    let client = Client::connect(paths, context).await.map_err(failed)?;
    let kind = Kind {
        group: String::new(),
        version: "v1".into(),
        kind: "Namespace".into(),
        plural: "namespaces".into(),
        namespaced: false,
        watchable: true,
    };
    let query = groove_kube::Query {
        kind: &kind,
        namespace: None,
        selector: None,
        whole: false,
    };
    let mut page = client.page(query, None).await.map_err(failed)?;
    let mut names: Vec<String> = page.rows.drain(..).map(|row| row.name).collect();
    while let Some(next) = page.next.take() {
        page = client.page(query, Some(&next)).await.map_err(failed)?;
        names.extend(page.rows.drain(..).map(|row| row.name));
    }
    names.sort();
    Ok(names)
}
