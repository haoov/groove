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

/// A host that answers the repository, the viewer, the assignment, and one mutation.
async fn host(mutation: &str, answered: serde_json::Value) -> MockServer {
    let server = MockServer::start().await;
    let fixed = [
        (
            "defaultBranchRef",
            serde_json::json!({ "repository": { "id": "REPO_1", "defaultBranchRef": { "name": "main" } } }),
        ),
        (
            "viewer { id login }",
            serde_json::json!({ "viewer": { "id": "U_1", "login": "haoov" } }),
        ),
        (
            "addAssigneesToAssignable",
            serde_json::json!({ "addAssigneesToAssignable": { "clientMutationId": null } }),
        ),
    ];
    for (asked, data) in fixed {
        Mock::given(method("POST"))
            .and(body_string_contains(asked))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({ "data": data })),
            )
            .mount(&server)
            .await;
    }
    Mock::given(method("POST"))
        .and(body_string_contains(mutation))
        .respond_with(ResponseTemplate::new(200).set_body_json(answered))
        .mount(&server)
        .await;
    server
}

/// What a mutation answers with: the MR under its own key.
fn wrote(key: &str, pr: serde_json::Value) -> serde_json::Value {
    serde_json::json!({ "data": { key: { "pullRequest": pr } } })
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

/// Plain git for setting a scene, with a repo-local identity and no signing.
fn sh(dir: &std::path::Path, args: &[&str]) {
    let out = std::process::Command::new("git")
        .args(["-c", "user.email=t@t", "-c", "user.name=T"])
        .args(["-c", "commit.gpgsign=false"])
        .args(args)
        .current_dir(dir)
        .env("LC_ALL", "C")
        .output()
        .expect("git runs");
    assert!(out.status.success(), "git {args:?}");
}

/// A clone of an origin whose only branch is `main`, `origin/HEAD` on it.
fn cloned(root: &std::path::Path) -> std::path::PathBuf {
    let (origin, work) = (root.join("origin.git"), root.join("work"));
    sh(root, &["init", "-q", "--bare", "-b", "main", "origin.git"]);
    sh(root, &["clone", "-q", &origin.to_string_lossy(), "work"]);
    sh(&work, &["commit", "-q", "--allow-empty", "-m", "one"]);
    sh(&work, &["push", "-q", "origin", "HEAD:main"]);
    sh(&work, &["remote", "set-head", "origin", "main"]);
    work
}

#[tokio::test]
async fn an_mr_targets_the_base_the_diff_is_read_against() {
    let root = tempfile::tempdir().unwrap();
    let mut at = worktree();
    let work = cloned(root.path()).to_string_lossy().into_owned();
    (at.path, at.base_ref) = (work, Some("gone".into()));
    let server = host("\"base\":\"main\"", wrote("createPullRequest", pr("OPEN"))).await;
    let (repo, service) = github(&server).await;
    let opened = service.open_mr(&repo, &at, &text()).await;
    assert!(opened.is_ok(), "{opened:?}");
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
    let server = host("createPullRequest", wrote("createPullRequest", pr("OPEN"))).await;
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
