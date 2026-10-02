//! The review queue: the MRs asked of the viewer, and the review each one shows them.

use super::*;

#[tokio::test]
async fn the_review_queue_reads_the_mrs_asked_of_this_user() {
    let reply = serde_json::json!({ "data": { "currentUser": {
        "username": "rsabbah",
        "reviewRequestedMergeRequests": { "nodes": [{
            "iid": "12",
            "title": "feat: a thing",
            "webUrl": "https://gitlab.example.com/g/p/-/merge_requests/12",
            "draft": true,
            "updatedAt": "2026-09-20T08:00:00Z",
            "sourceBranch": "feat/thing",
            "targetBranch": "main",
            "approved": false,
            "author": { "username": "someone" },
            "reviewers": { "nodes": [{
                "username": "rsabbah", "mergeRequestInteraction": { "reviewState": "REVIEWED" }
            }]},
            "project": { "fullPath": "g/p" }
        }]}
    }}});
    let (_server, gitlab) = gitlab(reply).await;
    let queue = gitlab.review_queue().await.expect("the queue answers");
    assert_eq!(queue.len(), 1);
    let one = &queue[0];
    assert_eq!(one.iid, 12);
    assert_eq!(one.project, "g/p");
    assert_eq!(one.forge, groove_types::Forge::Gitlab);
    assert!(one.draft);
    assert!(!one.approved);
    assert_eq!(one.review, Some(groove_types::ReviewState::Commented));
}

/// An MR someone else approved, the viewer's own state `mine`.
fn approved_by_another(mine: &str) -> serde_json::Value {
    let node = serde_json::json!({
        "iid": "13", "title": "fix: b", "webUrl": "https://gitlab.example.com/g/p/-/merge_requests/13",
        "draft": false, "updatedAt": "2026-09-20T08:00:00Z",
        "sourceBranch": "fix/b", "targetBranch": "main", "approved": true,
        "author": { "username": "someone" },
        "reviewers": { "nodes": [
            { "username": "other", "mergeRequestInteraction": { "reviewState": "APPROVED" } },
            { "username": "rsabbah", "mergeRequestInteraction": { "reviewState": mine } }
        ]},
        "project": { "fullPath": "g/p" }
    });
    serde_json::json!({ "data": { "currentUser": {
        "username": "rsabbah",
        "reviewRequestedMergeRequests": { "nodes": [node] }
    }}})
}

#[tokio::test]
async fn a_review_still_asked_of_the_viewer_shows_as_requested_whoever_approved() {
    let (_server, gitlab) = gitlab(approved_by_another("UNREVIEWED")).await;
    let queue = gitlab.review_queue().await.expect("the queue answers");
    assert_eq!(queue[0].review, Some(ReviewState::Requested));
}

#[tokio::test]
async fn once_the_viewer_reviewed_it_shows_what_every_reviewer_said() {
    let (_server, gitlab) = gitlab(approved_by_another("REVIEWED")).await;
    let queue = gitlab.review_queue().await.expect("the queue answers");
    assert_eq!(queue[0].review, Some(ReviewState::Commented));
}
