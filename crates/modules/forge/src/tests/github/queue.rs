//! The review queue: the PRs asked of the viewer, and the review each one shows them.

use super::*;

/// One row of the review search, as GitHub answers it.
fn asked(number: u64, updated: &str, decision: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "number": number,
        "title": "fix(forge): read the checks",
        "url": format!("https://github.com/acme/groove/pull/{number}"),
        "isDraft": false,
        "updatedAt": updated,
        "reviewDecision": decision,
        "author": { "login": "someone" },
        "headRefName": "fix/checks",
        "headRefOid": "cafe1234",
        "id": "PR_node",
        "baseRefName": "main",
        "repository": { "nameWithOwner": "acme/groove" }
    })
}

#[tokio::test]
async fn the_review_queue_answers_newest_first_and_skips_what_is_not_an_mr() {
    let mut seven = asked(7, "2026-09-18T08:00:00Z", serde_json::Value::Null);
    seven["latestReviews"] = serde_json::json!({ "nodes": [
        { "state": "COMMENTED", "author": { "login": "passer-by" } }
    ]});
    let reply = serde_json::json!({ "data": { "search": { "nodes": [
        seven,
        serde_json::json!({}),
        asked(9, "2026-09-20T08:00:00Z", serde_json::json!("APPROVED")),
    ]}}});
    let (_server, github) = github(reply).await;
    let queue = github.review_queue().await.expect("the search answers");
    let numbers: Vec<u64> = queue.iter().map(|mr| mr.iid).collect();
    assert_eq!(numbers, [9, 7], "newest first, and the bare node is gone");
    let first = &queue[0];
    assert_eq!(first.project, "acme/groove");
    assert_eq!(first.author, "someone");
    assert_eq!(first.source_branch, "fix/checks");
    assert_eq!(first.target_branch, "main");
    assert_eq!(first.forge, groove_types::Forge::Github);
    assert!(first.approved, "its decision is approved");
    assert!(!first.draft);
    assert_eq!(
        first.local_path, None,
        "the pool answers that, not the forge"
    );
    assert!(!queue[1].approved);
    assert_eq!(first.review, Some(groove_types::ReviewState::Approved));
    assert_eq!(queue[1].review, Some(groove_types::ReviewState::Commented));
}

#[tokio::test]
async fn a_review_asked_again_of_the_viewer_shows_as_requested_over_an_approval() {
    let mut nine = asked(9, "2026-09-20T08:00:00Z", serde_json::json!("APPROVED"));
    nine["latestReviews"] = serde_json::json!({ "nodes": [
        { "state": "APPROVED", "author": { "login": "other" } },
        { "state": "COMMENTED", "author": { "login": "rsabbah" } }
    ]});
    nine["reviewRequests"] = serde_json::json!({ "nodes": [
        { "requestedReviewer": { "login": "rsabbah" } }
    ]});
    let reply = serde_json::json!({ "data": {
        "viewer": { "login": "rsabbah" },
        "search": { "nodes": [nine] }
    }});
    let (_server, github) = github(reply).await;
    let queue = github.review_queue().await.expect("the search answers");
    assert_eq!(queue[0].review, Some(ReviewState::Requested));
}
