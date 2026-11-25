//! A commit as Argo CD's repo server reads it: its author, its date and its message.

use groove_types::{RevisionMeta, Timestamp};
use serde_json::Value;

/// The metadata of `revision` in the Application `namespace/name`'s source `source` of several.
pub fn revision_path(namespace: &str, name: &str, revision: &str, source: Option<usize>) -> String {
    let path = format!(
        "/api/v1/applications/{name}/revisions/{revision}/metadata?appNamespace={namespace}"
    );
    match source {
        Some(at) => format!("{path}&sourceIndex={at}"),
        None => path,
    }
}

/// The revision's metadata; `Err` holds what Argo CD said when the body is none.
pub fn revision(body: &str) -> Result<RevisionMeta, String> {
    let json: Value = serde_json::from_str(body).map_err(|_| first_line(body))?;
    if json.get("date").is_none() && json.get("author").is_none() {
        let said = json["message"].as_str().map(String::from);
        return Err(said.unwrap_or_else(|| first_line(body)));
    }
    let text = |key: &str| json[key].as_str().unwrap_or_default();
    Ok(RevisionMeta {
        author: author(text("author")),
        date: Timestamp::parse(text("date")).ok(),
        message: text("message")
            .lines()
            .next()
            .unwrap_or_default()
            .to_string(),
    })
}

/// `Jane Doe <jane@example.com>` read as `Jane Doe`.
fn author(said: &str) -> String {
    said.split(" <").next().unwrap_or(said).trim().to_string()
}

fn first_line(body: &str) -> String {
    body.lines().next().unwrap_or_default().to_string()
}
