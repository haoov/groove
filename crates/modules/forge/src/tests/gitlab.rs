//! GitLab: what one reply says, and what a write leaves behind.

use groove_token::Token;
use groove_types::{CiState, MrState, Repo, RepoId, ReviewState};
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::Gitlab;

fn repo() -> Repo {
    Repo {
        id: RepoId::new("r1"),
        host: "gitlab.example.com".into(),
        group_path: "wiremind/devops".into(),
        project: "overwhelm".into(),
        local_path: "/main/overwhelm".into(),
    }
}

/// One merge request as GitLab answers for it.
fn mr(state: &str) -> serde_json::Value {
    serde_json::json!({
        "id": "gid://gitlab/MergeRequest/99",
        "iid": "7",
        "title": "fix(forge): read the pipeline",
        "description": "## What\nthe pipeline",
        "state": state,
        "draft": false,
        "webUrl": "https://gitlab.example.com/wiremind/devops/overwhelm/-/merge_requests/7",
        "createdAt": "2026-09-18T08:00:00Z",
        "updatedAt": "2026-09-19T09:30:00Z",
        "sourceBranch": "fix/pipeline",
        "targetBranch": "main",
        "author": { "username": "rsabbah" },
        "approved": true,
        "approvedBy": { "nodes": [{ "username": "reviewer" }] },
        "reviewers": { "nodes": [
            { "username": "reviewer",
              "mergeRequestInteraction": { "reviewState": "APPROVED" } },
            { "username": "awaited",
              "mergeRequestInteraction": { "reviewState": "UNREVIEWED" } }
        ]},
        "headPipeline": {
            "status": "FAILED",
            "finishedAt": "2026-09-19T09:25:00Z",
            "path": "/wiremind/devops/overwhelm/-/pipelines/1234"
        },
        "discussions": { "nodes": [{
            "id": "gid://gitlab/Discussion/abc",
            "resolved": false,
            "notes": { "nodes": [{
                "body": "issue: this drops the error",
                "createdAt": "2026-09-19T09:00:00Z",
                "resolvable": true,
                "resolved": false,
                "author": { "username": "reviewer" },
                "position": {
                    "newPath": "src/lib.rs", "newLine": 12,
                    "oldPath": "src/lib.rs", "oldLine": null
                }
            }]}
        }]}
    })
}

/// A server answering every call with `reply`, and a client on it.
async fn gitlab(reply: serde_json::Value) -> (MockServer, Gitlab) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(reply))
        .mount(&server)
        .await;
    let host = format!("http://{}", server.address());
    let gitlab = Gitlab::with_token(&host, Token::Fixed("t".into())).expect("a client");
    (server, gitlab)
}

fn by_branch(nodes: Vec<serde_json::Value>) -> serde_json::Value {
    serde_json::json!({ "data": {
        "currentUser": { "username": "rsabbah" },
        "project": { "mergeRequests": { "nodes": nodes } }
    }})
}

#[tokio::test]
async fn the_mr_of_a_branch_comes_back_with_its_fields() {
    let (_server, gitlab) = gitlab(by_branch(vec![mr("opened")])).await;
    let read = gitlab
        .open_mr(&repo(), "fix/pipeline")
        .await
        .expect("the query answers")
        .expect("one open mr");
    assert_eq!(read.number, "7", "gitlab counts by iid");
    assert_eq!(read.node, "gid://gitlab/MergeRequest/99");
    assert_eq!(read.details.title, "fix(forge): read the pipeline");
    assert_eq!(read.details.author, "rsabbah");
    assert_eq!(read.details.state, MrState::Open, "opened is open");
    assert_eq!(read.details.target_branch, "main");
}

#[tokio::test]
async fn a_branch_with_no_open_mr_answers_nothing() {
    let (_server, gitlab) = gitlab(by_branch(vec![])).await;
    assert!(
        gitlab
            .open_mr(&repo(), "fix/pipeline")
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn the_pipeline_on_its_head_is_the_ci_of_the_row() {
    let (_server, gitlab) = gitlab(by_branch(vec![mr("opened")])).await;
    let read = gitlab.open_mr(&repo(), "b").await.unwrap().unwrap();
    let ci = read.ci.expect("a pipeline");
    assert_eq!(ci.state, CiState::Failed);
    assert!(
        ci.url.starts_with("http://127.0.0.1:")
            && ci
                .url
                .ends_with("/wiremind/devops/overwhelm/-/pipelines/1234"),
        "the pipeline's own page on the host: {}",
        ci.url
    );
    assert_eq!(
        ci.finished_at,
        Some(groove_types::Timestamp::parse("2026-09-19T09:25:00Z").unwrap())
    );
}

#[tokio::test]
async fn an_mr_with_no_pipeline_reports_no_ci() {
    let mut bare = mr("opened");
    bare["headPipeline"] = serde_json::Value::Null;
    let (_server, gitlab) = gitlab(by_branch(vec![bare])).await;
    let read = gitlab.open_mr(&repo(), "b").await.unwrap().unwrap();
    assert!(read.ci.is_none());
}

#[tokio::test]
async fn a_reviewers_own_state_says_what_they_have_said() {
    let (_server, gitlab) = gitlab(by_branch(vec![mr("opened")])).await;
    let read = gitlab.open_mr(&repo(), "b").await.unwrap().unwrap();
    let said: Vec<(&str, ReviewState)> = read
        .details
        .reviewers
        .iter()
        .map(|one| (one.name.as_str(), one.state))
        .collect();
    assert_eq!(
        said,
        vec![
            ("reviewer", ReviewState::Approved),
            ("awaited", ReviewState::Requested),
        ]
    );
    let approval = read.details.approval.expect("an approval");
    assert!(approval.approved);
    assert_eq!(approval.approved_by, vec!["reviewer".to_string()]);
    assert!(!approval.approved_by_me, "rsabbah wrote it");
}

#[tokio::test]
async fn a_discussion_reads_as_a_thread_at_its_line() {
    let (_server, gitlab) = gitlab(by_branch(vec![mr("opened")])).await;
    let read = gitlab.open_mr(&repo(), "b").await.unwrap().unwrap();
    assert_eq!(read.threads.len(), 1);
    let note = &read.threads[0].notes[0];
    assert_eq!(note.author, "reviewer");
    assert!(note.resolvable);
    let at = note.position.clone().expect("a position");
    assert_eq!(at.new_path.as_deref(), Some("src/lib.rs"));
    assert_eq!(at.new_line, Some(12));
    assert_eq!(at.old_line, None, "the note is on the new side only");
}

#[tokio::test]
async fn what_a_mutation_refuses_is_the_writes_failure() {
    let reply = serde_json::json!({ "data": {
        "currentUser": { "username": "rsabbah" },
        "mergeRequestCreate": {
            "errors": ["Another open merge request already exists for this source branch"],
            "mergeRequest": null
        }
    }});
    let (_server, gitlab) = gitlab(reply).await;
    let refused = gitlab
        .open_new(
            &repo(),
            crate::Proposed {
                head: "fix/pipeline",
                base: Some("main"),
                title: "fix: one",
                body: "why",
            },
        )
        .await
        .expect_err("the write is refused");
    assert!(refused.to_string().contains("already exists"), "{refused}");
}

#[tokio::test]
async fn an_mr_opened_answers_with_the_mr_itself() {
    let reply = serde_json::json!({ "data": {
        "currentUser": { "username": "rsabbah" },
        "mergeRequestCreate": { "errors": [], "mergeRequest": mr("opened") }
    }});
    let (_server, gitlab) = gitlab(reply).await;
    let opened = gitlab
        .open_new(
            &repo(),
            crate::Proposed {
                head: "fix/pipeline",
                base: Some("main"),
                title: "fix: one",
                body: "why",
            },
        )
        .await
        .expect("it is opened");
    assert_eq!(opened.number, "7");
    assert_eq!(opened.details.state, MrState::Open);
}

#[tokio::test]
async fn closing_answers_with_the_mr_closed() {
    let reply = serde_json::json!({ "data": {
        "currentUser": { "username": "rsabbah" },
        "mergeRequestUpdate": { "errors": [], "mergeRequest": mr("closed") }
    }});
    let (_server, gitlab) = gitlab(reply).await;
    let shut = gitlab.shut_mr(&repo(), "7").await.expect("it is closed");
    assert_eq!(shut.details.state, MrState::Closed);
}

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
}
