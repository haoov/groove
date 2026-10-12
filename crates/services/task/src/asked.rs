//! What the sources are asked: list, fetch, status, hours, template, schema, connect.

use groove_types::{ProviderId, Result, Task, TaskKey};

use crate::{Fetched, Github, Notion, Source};

/// Every task the sources hold, in the order they answer.
pub async fn list(sources: &[Source]) -> Result<Vec<Task>> {
    let mut tasks = Vec::new();
    for source in sources {
        tasks.extend(source.list().await?);
    }
    Ok(tasks)
}

/// Sets one task's status to what its source calls this intent.
pub async fn set_status(
    sources: &[Source],
    key: &TaskKey,
    intent: groove_types::StatusIntent,
) -> Result<String> {
    Ok(source_of(sources, key)?.set_status(key, intent).await?)
}

/// Adds hours to what the source holds against one task. Returns its new total.
pub async fn log_hours(sources: &[Source], key: &TaskKey, hours: f32) -> Result<f32> {
    Ok(source_of(sources, key)?.log_hours(key, hours).await?)
}

/// The template one source holds, or nothing when it holds none.
pub async fn template(sources: &[Source], which: Option<ProviderId>) -> Result<Option<String>> {
    let Some(source) = pick(sources, which) else {
        return Err(unpicked(sources, which));
    };
    Ok(source.template().await?)
}

/// Why no source answers to `which`.
pub(crate) fn unpicked(sources: &[Source], which: Option<ProviderId>) -> groove_types::Error {
    let named: Vec<&str> = sources.iter().map(|one| one.id().as_str()).collect();
    groove_types::Error::invalid(match (which, named.len()) {
        (_, 0) => "no source is set up".to_string(),
        (Some(id), _) => format!(
            "{} is not set up; set up: {}",
            id.as_str(),
            named.join(", ")
        ),
        (None, _) => format!(
            "several sources are set up; name the provider: {}",
            named.join(", ")
        ),
    })
}

/// The source named, or the only one there is.
fn pick(sources: &[Source], which: Option<ProviderId>) -> Option<&Source> {
    match which {
        Some(id) => sources.iter().find(|one| one.id() == id),
        None => sources.first().filter(|_| sources.len() == 1),
    }
}

/// A Notion source, once its token reads the database.
pub async fn connect_notion(
    token: &str,
    database_id: &str,
    user_id: &str,
) -> Result<groove_types::NotionConfig> {
    Ok(Notion::connect(token, database_id, user_id).await?)
}

/// The properties one source holds, for its names to be mapped onto.
pub async fn schema(sources: &[Source], which: ProviderId) -> Result<Vec<groove_types::Property>> {
    let Some(source) = sources.iter().find(|one| one.id() == which) else {
        return Err(groove_types::Error::invalid(format!(
            "{} is not on",
            which.label()
        )));
    };
    Ok(source.schema().await?)
}

/// A GitHub source, once the host answers the token `gh` holds for it.
pub async fn connect_github(host: &str) -> Result<groove_types::GithubConfig> {
    Ok(Github::connect(host).await?)
}

/// One task and its body, read again from the source that owns it.
pub async fn fetch(sources: &[Source], key: &TaskKey) -> Result<Fetched> {
    Ok(source_of(sources, key)?.fetch(key).await?)
}

/// The source that owns the key, of the ones the config turned on.
fn source_of<'a>(sources: &'a [Source], key: &TaskKey) -> Result<&'a Source> {
    sources
        .iter()
        .find(|source| source.id() == key.provider())
        .ok_or_else(|| groove_types::Error::invalid(format!("no source for {}", key.external_id())))
}
