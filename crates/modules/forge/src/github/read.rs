//! What one reply says: the MR itself, and the parts each file below reads.

mod ci;
pub(super) mod queue;
mod reviews;
mod threads;

use groove_types::{MrDetails, MrState};

use crate::Snapshot;

/// One MR out of the reply, or nothing where the query found none.
pub(super) fn snapshot(pr: &serde_json::Value, me: &str) -> Option<Snapshot> {
    let number = pr["number"].as_i64()?;
    Some(Snapshot {
        node: text(&pr["id"]),
        head: text(&pr["headRefOid"]),
        number: number.to_string(),
        details: details(pr, me),
        ci: ci::status(pr),
        threads: threads::all(pr),
    })
}

fn details(pr: &serde_json::Value, me: &str) -> MrDetails {
    MrDetails {
        title: text(&pr["title"]),
        description: text(&pr["body"]),
        author: text(&pr["author"]["login"]),
        source_branch: text(&pr["headRefName"]),
        target_branch: text(&pr["baseRefName"]),
        state: state(&pr["state"]),
        draft: pr["isDraft"].as_bool().unwrap_or_default(),
        created_at: at(&pr["createdAt"]).unwrap_or_default(),
        updated_at: at(&pr["updatedAt"]).unwrap_or_default(),
        web_url: text(&pr["url"]),
        approval: Some(reviews::approval(pr, me)),
        reviewers: reviews::all(pr),
    }
}

/// GitHub's own word for the state; a pull request it does not name is open.
fn state(value: &serde_json::Value) -> MrState {
    match value.as_str() {
        Some("MERGED") => MrState::Merged,
        Some("CLOSED") => MrState::Closed,
        _ => MrState::Open,
    }
}

pub(super) use groove_types::json::{at, text};
