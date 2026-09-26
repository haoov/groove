//! The writes: an MR opened, written again and closed, each one written down.

use groove_types::MrState;
use wiremock::matchers::{body_string_contains, method};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::delivery::{answer, github, pr, worktree};
use crate::Text;

fn text() -> Text {
    Text {
        title: "fix: one".into(),
        body: "why it is\n\nTask: github.com/haoov/groove#50".into(),
    }
}

/// A host that answers the repository query, and one mutation.
async fn host(mutation: &str, answered: serde_json::Value) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(body_string_contains("defaultBranchRef"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": { "repository": { "id": "REPO_1", "defaultBranchRef": { "name": "main" } } }
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(body_string_contains(mutation))
        .respond_with(ResponseTemplate::new(200).set_body_json(answered))
        .mount(&server)
        .await;
    server
}

/// What a mutation answers with: the viewer, and the MR under its own key.
fn wrote(key: &str, pr: serde_json::Value) -> serde_json::Value {
    serde_json::json!({ "data": {
        "viewer": { "login": "haoov" },
        key: { "pullRequest": pr }
    }})
}

#[tokio::test]
async fn an_mr_opened_is_written_down_as_the_forge_answers() {
    let server = host("createPullRequest", wrote("createPullRequest", pr("OPEN"))).await;
    let (repo, service) = github(&server).await;
    let opened = service
        .open_mr(&repo, &worktree(), &text())
        .await
        .expect("the mr is opened");
    assert_eq!(opened.mr.remote_id, "7");
    assert_eq!(opened.mr.state, MrState::Open);
    let stored = service.stored(&worktree().id).await.unwrap();
    assert_eq!(stored.map(|mr| mr.remote_id), Some("7".to_string()));
}

#[tokio::test]
async fn a_worktree_with_no_mr_cannot_have_one_written_or_closed() {
    let server = host("updatePullRequest", wrote("updatePullRequest", pr("OPEN"))).await;
    let (repo, service) = github(&server).await;
    let refused = service
        .edit_mr(&repo, &worktree(), &text())
        .await
        .expect_err("it has none");
    assert_eq!(refused.kind, groove_types::ErrorKind::NotFound);
    assert!(service.close_mr(&repo, &worktree()).await.is_err());
}

#[tokio::test]
async fn closing_leaves_the_row_saying_so() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(body_string_contains("defaultBranchRef"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": { "repository": { "id": "REPO_1", "defaultBranchRef": { "name": "main" } } }
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(body_string_contains("createPullRequest"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(wrote("createPullRequest", pr("OPEN"))),
        )
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(body_string_contains("pullRequest(number"))
        .respond_with(ResponseTemplate::new(200).set_body_json(answer("pullRequest", pr("OPEN"))))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(body_string_contains("closePullRequest"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(wrote("closePullRequest", pr("CLOSED"))),
        )
        .mount(&server)
        .await;

    let (repo, service) = github(&server).await;
    service
        .open_mr(&repo, &worktree(), &text())
        .await
        .expect("opened");
    let shut = service.close_mr(&repo, &worktree()).await.expect("closed");
    assert_eq!(shut.mr.state, MrState::Closed);
    let stored = service.stored(&worktree().id).await.unwrap();
    assert_eq!(stored.map(|mr| mr.state), Some(MrState::Closed));
}
