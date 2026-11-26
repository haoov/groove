//! What a verdict and a comment on lines send to the forge.

use groove_mrs::Stored as _;
use groove_types::{Anchor, ReviewVerdict};
use wiremock::matchers::{body_string_contains, method};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::delivery::{repo, service, worktree};
use crate::{Said, Service};

/// The repo on the mock host, and the service reaching it as GitLab.
fn gitlab(server: &MockServer, service: Service) -> (groove_types::Repo, Service) {
    let at = repo(&format!("http://{}", server.address()));
    let service = service.connecting(|repo| {
        let token = groove_forge::Token::Fixed("t".into());
        Ok(crate::Remote::Gitlab(groove_forge::Gitlab::with_token(
            &repo.host, token,
        )?))
    });
    (at, service)
}

/// The MR row the writes address, already held for the worktree.
async fn held(service: &Service) {
    sqlx::query(
        "INSERT INTO mrs (id, worktree_id, platform, remote_id, url, state)
         VALUES ('m1', 'w1', 'gitlab', '7', 'https://example.com/7', 'open')",
    )
    .execute(service.store().db().pool())
    .await
    .expect("the row is seeded");
}

/// One MR as a read by its number answers.
fn mr() -> serde_json::Value {
    serde_json::json!({ "data": {
        "currentUser": { "username": "you" },
        "project": { "mergeRequest": {
            "id": "gid://gitlab/MergeRequest/99",
            "iid": "7",
            "title": "fix: one",
            "description": "",
            "state": "opened",
            "draft": false,
            "webUrl": "https://example.com/7",
            "createdAt": "2026-09-18T08:00:00Z",
            "updatedAt": "2026-09-19T09:30:00Z",
            "diffHeadSha": "cafe1234",
            "sourceBranch": "fix/one",
            "targetBranch": "main",
            "author": { "username": "haoov" },
            "approved": false,
            "approvedBy": { "nodes": [] },
            "reviewers": { "nodes": [] },
            "headPipeline": null,
            "discussions": { "nodes": [] }
        }}
    }})
}

#[tokio::test]
async fn a_comment_on_lines_goes_up_on_the_lines_the_forge_counts_from_one() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(body_string_contains("createLatestDiffNote"))
        .and(body_string_contains(r#""from":10"#))
        .and(body_string_contains(r#""to":12"#))
        .respond_with(ResponseTemplate::new(200).set_body_json(mr()))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(body_string_contains("createLatestDiffNote"))
        .respond_with(ResponseTemplate::new(500).set_body_string("wrong lines"))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mr()))
        .mount(&server)
        .await;
    let service = service().await;
    held(&service).await;
    let (repo, service) = gitlab(&server, service);
    let anchor = Anchor::numbered("src/lib.rs", 10, Some(12)).expect("lines");
    service
        .comment_lines(&repo, &worktree().id, &anchor, "issue: it leaks")
        .await
        .expect("the comment is posted");
}

#[tokio::test]
async fn a_verdict_goes_up_with_its_words() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mr()))
        .mount(&server)
        .await;
    let service = service().await;
    held(&service).await;
    let (repo, service) = gitlab(&server, service);
    let said = Said {
        verdict: ReviewVerdict::Approve,
        body: "looks right",
    };
    service
        .review(&repo, &worktree().id, said)
        .await
        .expect("the review is posted");
}
