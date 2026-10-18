//! Writes to a cluster: who may make one, and the write itself.

use std::path::PathBuf;

use groove_types::{Attached, Error, Result, Target};
use serde_json::Value;

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

pub async fn patch(
    paths: Vec<PathBuf>,
    target: Target,
    body: Value,
    version: Option<String>,
    dry_run: bool,
) -> Result<Value> {
    groove_objects::patch(&paths, &target, &body, version.as_deref(), dry_run).await
}
