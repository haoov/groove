//! Writes to a cluster: who may make one, and the write itself.

use std::path::PathBuf;

use groove_objects::Preview;
use groove_types::{Attached, Error, KubeKind, Result, Target, Timestamp};
use serde_json::{Value, json};

/// An object as a human names it: its kind as `deployments`, `Deployment`, `deploy` or `deployments.apps`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asked {
    pub context: String,
    pub kind: String,
    pub namespace: Option<String>,
    pub name: String,
    pub subresource: Option<String>,
}

/// Where a session may write: the context's settings and what the session holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Allowed {
    pub read_only: bool,
    pub held: Vec<Attached>,
}

/// Refused on a read-only context, or where the session does not hold the object.
pub fn writable(target: &Target, read_only: bool, held: &[Attached]) -> Result<()> {
    let context = &target.context;
    if read_only {
        return Err(Error::invalid(format!(
            "{context} is read-only: turn it off in Settings › Clusters to write"
        )));
    }
    let on: Vec<&Attached> = held.iter().filter(|one| &one.context == context).collect();
    if on.is_empty() {
        return Err(Error::invalid(format!(
            "the session holds no context {context}"
        )));
    }
    let holds = |one: &&Attached| match (&one.namespace, &target.namespace) {
        (None, _) => true,
        (Some(held), Some(namespace)) => held == namespace,
        (Some(_), None) => false,
    };
    if on.iter().any(holds) {
        return Ok(());
    }
    let why = match &target.namespace {
        Some(namespace) => {
            format!("the session does not hold the namespace {namespace} of {context}")
        }
        None => format!(
            "{} stands in no namespace: the session must hold the whole of {context}",
            target.name
        ),
    };
    Err(Error::invalid(why))
}

/// Whether a restart replaces the pods of `kind`: a workload rolling out a pod template.
pub fn restartable(kind: &KubeKind) -> bool {
    let rolled = ["Deployment", "StatefulSet", "DaemonSet"];
    kind.group == "apps" && rolled.contains(&kind.kind.as_str())
}

/// Whether `kind` scales through its `scale` subresource from a row.
pub fn scalable(kind: &KubeKind) -> bool {
    kind.group == "apps" && ["Deployment", "StatefulSet"].contains(&kind.kind.as_str())
}

/// The patch `kubectl scale` sends to `target`'s `scale` subresource.
pub fn scale(target: &mut Target, replicas: u32) -> Result<Value> {
    if !scalable(&target.kind) {
        let why = format!("a {} does not scale", target.kind.kind);
        return Err(Error::invalid(why));
    }
    target.subresource = Some("scale".into());
    Ok(json!({ "spec": { "replicas": replicas } }))
}

/// The patch `kubectl rollout restart` sends: the pod template stamped with `at`.
pub fn restart(kind: &KubeKind, at: Timestamp) -> Result<Value> {
    if !restartable(kind) {
        let why = format!("a {} has no pods to restart", kind.kind);
        return Err(Error::invalid(why));
    }
    let stamp = json!({ "kubectl.kubernetes.io/restartedAt": at.rfc3339() });
    Ok(json!({ "spec": { "template": { "metadata": { "annotations": stamp } } } }))
}

pub async fn patch(
    paths: Vec<PathBuf>,
    target: Target,
    body: Value,
    version: Option<String>,
    dry_run: bool,
) -> Result<Value> {
    groove_objects::patch(&paths, &target, &body, version.as_deref(), dry_run).await
}

pub async fn delete(paths: Vec<PathBuf>, target: Target, version: Option<String>) -> Result<()> {
    groove_objects::delete(&paths, &target, version.as_deref(), false).await
}

/// The object `asked` names among the kinds under `cache`, checked, then the patch tried on it.
pub async fn preview(
    paths: Vec<PathBuf>,
    cache: PathBuf,
    asked: Asked,
    allowed: Allowed,
    body: Value,
) -> Result<(Target, Preview)> {
    let target = resolved(&paths, &cache, asked, &allowed).await?;
    let preview = groove_objects::preview(&paths, &target, &body).await?;
    Ok((target, preview))
}

/// The object `asked` names, checked, then its delete tried on it.
pub async fn doomed(
    paths: Vec<PathBuf>,
    cache: PathBuf,
    asked: Asked,
    allowed: Allowed,
) -> Result<(Target, Preview)> {
    let target = resolved(&paths, &cache, asked, &allowed).await?;
    let preview = groove_objects::doomed(&paths, &target).await?;
    Ok((target, preview))
}

/// The object `asked` names among the kinds under `cache`, where the session may write.
async fn resolved(
    paths: &[PathBuf],
    cache: &std::path::Path,
    asked: Asked,
    allowed: &Allowed,
) -> Result<Target> {
    let kinds = groove_objects::kinds(paths, &asked.context, cache, false).await?;
    let target = target(&kinds, asked)?;
    writable(&target, allowed.read_only, &allowed.held)?;
    Ok(target)
}

/// The object `asked` names, on the one kind its words name.
pub(crate) fn target(kinds: &[KubeKind], asked: Asked) -> Result<Target> {
    let kind = kind_named(kinds, &asked.kind)?;
    let namespace = match (kind.namespaced, asked.namespace) {
        (true, None) => {
            return Err(Error::invalid(format!(
                "{} stand in a namespace: name it",
                kind.plural
            )));
        }
        (true, namespace) => namespace,
        (false, _) => None,
    };
    Ok(Target {
        context: asked.context,
        kind,
        namespace,
        name: asked.name,
        subresource: asked.subresource,
    })
}

fn kind_named(kinds: &[KubeKind], named: &str) -> Result<KubeKind> {
    let named = named.to_lowercase();
    let (word, group) = match named.split_once('.') {
        Some((word, group)) => (word, Some(group)),
        None => (named.as_str(), None),
    };
    let matches = |one: &&KubeKind| {
        let words = one.plural == word
            || one.kind.to_lowercase() == word
            || one.short_names.iter().any(|short| short == word);
        words && group.is_none_or(|group| one.group == group)
    };
    let found: Vec<&KubeKind> = kinds.iter().filter(matches).collect();
    let core = found.iter().find(|one| one.group.is_empty());
    match (found.as_slice(), core) {
        ([one], _) | ([_, _, ..], Some(one)) => Ok((*one).clone()),
        ([], _) => Err(Error::invalid(format!(
            "the cluster serves no kind {named}"
        ))),
        (many, None) => {
            let named: Vec<String> = many
                .iter()
                .map(|one| format!("{}.{}", one.plural, one.group))
                .collect();
            Err(Error::invalid(format!(
                "{word} names several kinds; name one: {}",
                named.join(", ")
            )))
        }
    }
}
