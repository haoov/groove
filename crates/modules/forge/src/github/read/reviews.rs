//! Who has reviewed the MR, and who it is still waiting on.

use std::collections::HashMap;

use groove_types::{MrApproval, ReviewState, Reviewer, Timestamp};

use super::{at, text};

/// Every reviewer: the verdicts given, then the people still asked for one.
pub(super) fn all(pr: &serde_json::Value) -> Vec<Reviewer> {
    let mut out = given(pr);
    for (name, asked) in requested(pr) {
        if !out.iter().any(|one| one.name == name) {
            out.push(Reviewer {
                name,
                state: ReviewState::Requested,
                at: asked,
            });
        }
    }
    out
}

/// What the MR's approval stands at, and whether the viewer gave it.
pub(super) fn approval(pr: &serde_json::Value, me: &str) -> MrApproval {
    let approved_by: Vec<String> = given(pr)
        .into_iter()
        .filter(|one| one.state == ReviewState::Approved)
        .map(|one| one.name)
        .collect();
    MrApproval {
        approved: pr["reviewDecision"].as_str() == Some("APPROVED") || !approved_by.is_empty(),
        approved_by_me: approved_by.iter().any(|name| name == me),
        approved_by,
    }
}

/// The latest verdict of each reviewer who gave one.
fn given(pr: &serde_json::Value) -> Vec<Reviewer> {
    nodes(&pr["latestReviews"])
        .iter()
        .filter_map(|review| {
            Some(Reviewer {
                name: login(&review["author"])?,
                state: verdict(review["state"].as_str()?)?,
                at: at(&review["submittedAt"]),
            })
        })
        .collect()
}

/// Everyone a review is still asked of, and when they were last asked.
fn requested(pr: &serde_json::Value) -> Vec<(String, Option<Timestamp>)> {
    let asked = asked_at(pr);
    nodes(&pr["reviewRequests"])
        .iter()
        .filter_map(|request| login(&request["requestedReviewer"]))
        .map(|name| (name.clone(), asked.get(&name).copied()))
        .collect()
}

/// When each person was last asked, from the events that asked them.
fn asked_at(pr: &serde_json::Value) -> HashMap<String, Timestamp> {
    let mut out = HashMap::new();
    for event in nodes(&pr["timelineItems"]) {
        if let Some(name) = login(&event["requestedReviewer"])
            && let Some(when) = at(&event["createdAt"])
        {
            out.insert(name, when);
        }
    }
    out
}

/// GitHub's word for a verdict. One that only commented gave no verdict.
fn verdict(state: &str) -> Option<ReviewState> {
    match state {
        "APPROVED" => Some(ReviewState::Approved),
        "CHANGES_REQUESTED" => Some(ReviewState::ChangesRequested),
        "COMMENTED" => Some(ReviewState::Commented),
        _ => None,
    }
}

fn login(who: &serde_json::Value) -> Option<String> {
    let name = text(&who["login"]);
    (!name.is_empty()).then_some(name)
}

pub(super) fn nodes(list: &serde_json::Value) -> Vec<serde_json::Value> {
    list["nodes"].as_array().cloned().unwrap_or_default()
}
