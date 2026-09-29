//! The lines a diff note spans: GitLab's REST discussions name them, its GraphQL does not.

use groove_types::{MrThread, Repo};

use super::mr_url;

pub(super) fn url(host: &str, repo: &Repo, number: &str) -> String {
    format!("{}/discussions?per_page=100", mr_url(host, repo, number))
}

/// Whether any thread stands on a line, which a range could widen.
pub(super) fn wanted(threads: &[MrThread]) -> bool {
    let placed = threads.iter().flat_map(|one| one.notes.first());
    placed.into_iter().any(|note| note.position.is_some())
}

/// Each thread's notes given the last line of the range its discussion spans.
pub(super) fn spanned(threads: &mut [MrThread], discussions: &serde_json::Value) {
    let Some(discussions) = discussions.as_array() else {
        return;
    };
    for thread in threads {
        let id = thread.id.rsplit('/').next().unwrap_or_default();
        let found = discussions
            .iter()
            .find(|one| one["id"].as_str() == Some(id));
        let Some(end) = found.and_then(last_line) else {
            continue;
        };
        for position in thread
            .notes
            .iter_mut()
            .filter_map(|one| one.position.as_mut())
        {
            if position.new_line.is_some_and(|start| end > start) {
                position.end_new_line = Some(end);
            }
        }
    }
}

fn last_line(discussion: &serde_json::Value) -> Option<u32> {
    let range = &discussion["notes"][0]["position"]["line_range"];
    let line = range["end"]["new_line"].as_u64()?;
    u32::try_from(line).ok()
}
