//! A task as a human or an agent names it: the page's URL, the issue's, or its id.

use groove_types::{Error, ExternalId, Result, TaskKey};

#[cfg(test)]
mod tests;

/// The key a reference names: a GitHub issue URL or `host/owner/repo#n`, or a Notion URL or page id.
pub fn referenced(text: &str) -> Result<TaskKey> {
    let text = text.trim();
    if let Some(key) = issue(text) {
        return Ok(key);
    }
    if text.contains('#') {
        return TaskKey::parse(&ExternalId::new(text));
    }
    match page_id(text) {
        Some(page_id) => Ok(TaskKey::Notion { page_id }),
        None => Err(Error::invalid(format!("names no task: {text}"))),
    }
}

/// `https://<host>/<owner>/<repo>/issues/<number>`, as GitHub shows an issue.
fn issue(text: &str) -> Option<TaskKey> {
    let rest = text.split_once("://")?.1;
    let parts: Vec<&str> = rest.trim_end_matches('/').split('/').collect();
    let [host, owner, repo, "issues", number, ..] = parts[..] else {
        return None;
    };
    Some(TaskKey::Github {
        host: host.to_string(),
        owner: owner.to_string(),
        repo: repo.to_string(),
        number: number.parse().ok()?,
    })
}

/// The 32 hex characters a Notion page id is: its URL ends on them, dashed or not.
fn page_id(text: &str) -> Option<String> {
    let path = text.split(['?', '#']).next().unwrap_or(text);
    let tail = path
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or(path);
    let flat: String = tail.chars().filter(|one| *one != '-').collect();
    let id = flat.get(flat.len().checked_sub(32)?..)?;
    id.chars()
        .all(|one| one.is_ascii_hexdigit())
        .then(|| id.to_string())
}
