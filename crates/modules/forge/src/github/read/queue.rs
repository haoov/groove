//! The review queue: the merge requests the viewer is asked to look at.

use groove_types::{Forge, ReviewMr, review_of};

use super::reviews::nodes;
use super::{at, text};

/// One row a search answered with, or nothing where it is not a merge request.
pub(crate) fn asked(mr: &serde_json::Value) -> Option<ReviewMr> {
    let iid = mr["number"].as_u64()?;
    let project = text(&mr["repository"]["nameWithOwner"]);
    let approved = mr["reviewDecision"].as_str() == Some("APPROVED");
    if project.is_empty() {
        return None;
    }
    Some(ReviewMr {
        forge: Forge::Github,
        project,
        iid,
        title: text(&mr["title"]),
        author: text(&mr["author"]["login"]),
        source_branch: text(&mr["headRefName"]),
        target_branch: text(&mr["baseRefName"]),
        draft: mr["isDraft"].as_bool().unwrap_or_default(),
        web_url: text(&mr["url"]),
        updated_at: at(&mr["updatedAt"]).unwrap_or_default(),
        local_path: None,
        approved,
        review: review_of(&super::reviews::all(mr), approved),
    })
}

/// Every merge request in the reply, newest first.
pub(crate) fn queue(reply: &serde_json::Value) -> Vec<ReviewMr> {
    let mut out: Vec<ReviewMr> = nodes(&reply["data"]["search"])
        .iter()
        .filter_map(asked)
        .collect();
    out.sort_by_key(|mr| std::cmp::Reverse(mr.updated_at.seconds()));
    out
}
