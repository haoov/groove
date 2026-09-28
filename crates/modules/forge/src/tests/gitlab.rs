//! GitLab: what one reply says, and what a write leaves behind.

use groove_token::Token;
use groove_types::{CiState, MrState, Repo, RepoId, ReviewState};
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::Gitlab;

pub(super) fn repo() -> Repo {
    Repo {
        id: RepoId::new("r1"),
        host: "gitlab.example.com".into(),
        group_path: "wiremind/devops".into(),
        project: "overwhelm".into(),
        local_path: "/main/overwhelm".into(),
    }
}

/// One merge request as GitLab answers for it.
pub(super) fn mr(state: &str) -> serde_json::Value {
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
        "diffHeadSha": "cafe1234",
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
pub(super) async fn gitlab(reply: serde_json::Value) -> (MockServer, Gitlab) {
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
    let (server, gitlab) = gitlab(reply).await;
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
    let queries: Vec<String> = sent(&server)
        .await
        .iter()
        .filter_map(|one| one["query"].as_str().map(str::to_string))
        .collect();
    let mutation = queries
        .iter()
        .find(|one| one.contains("mergeRequestCreate"));
    let mutation = mutation.expect("the write was sent");
    assert!(
        !mutation.contains("currentUser"),
        "GitLab refuses it there: {mutation}"
    );
    assert!(
        queries
            .iter()
            .any(|one| one.starts_with("query") && one.contains("currentUser"))
    );
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

/// One MR as a read by its number answers, and the mutations after it.
fn by_iid(one: serde_json::Value) -> serde_json::Value {
    serde_json::json!({ "data": {
        "currentUser": { "username": "rsabbah" },
        "project": { "mergeRequest": one },
        "createLatestDiffNote": { "errors": [], "note": { "id": "gid://gitlab/Note/1" } },
        "createNote": { "errors": [], "note": { "id": "gid://gitlab/Note/2" } },
        "discussionToggleResolve": { "errors": [], "discussion": { "resolved": true } },
        "mergeRequestRequestChanges": { "errors": [] }
    }})
}

/// Every body the server was sent.
pub(super) async fn sent(server: &MockServer) -> Vec<serde_json::Value> {
    server
        .received_requests()
        .await
        .unwrap_or_default()
        .iter()
        .filter_map(|one| serde_json::from_slice(&one.body).ok())
        .collect()
}

/// The variables of the call that carried `mutation`.
fn variables(sent: &[serde_json::Value], mutation: &str) -> serde_json::Value {
    sent.iter()
        .find(|one| one["query"].as_str().is_some_and(|q| q.contains(mutation)))
        .map(|one| one["variables"].clone())
        .unwrap_or_default()
}

#[tokio::test]
async fn a_note_is_posted_on_the_lines_it_covers() {
    let (server, gitlab) = gitlab(by_iid(mr("opened"))).await;
    gitlab
        .note_on(
            &repo(),
            "7",
            crate::Posted {
                path: "src/lib.rs",
                from: 12,
                to: 14,
                body: "issue: this leaks",
            },
        )
        .await
        .expect("the note is posted");
    let at = variables(&sent(&server).await, "createLatestDiffNote");
    assert_eq!(at["mr"], "gid://gitlab/MergeRequest/99");
    assert_eq!(at["head"], "cafe1234", "the diff the note hangs on");
    assert_eq!(at["path"], "src/lib.rs");
    assert_eq!(
        (at["from"].as_u64(), at["to"].as_u64()),
        (Some(12), Some(14))
    );
    assert_eq!(at["body"], "issue: this leaks");
}

#[tokio::test]
async fn a_reply_goes_under_the_discussion_it_answers() {
    let (server, gitlab) = gitlab(by_iid(mr("opened"))).await;
    gitlab
        .reply_to(&repo(), "7", "gid://gitlab/Discussion/abc", "fixed")
        .await
        .expect("the reply is posted");
    let at = variables(&sent(&server).await, "createNote");
    assert_eq!(at["mr"], "gid://gitlab/MergeRequest/99");
    assert_eq!(at["thread"], "gid://gitlab/Discussion/abc");
    assert_eq!(at["body"], "fixed");
}

#[tokio::test]
async fn a_discussion_resolves_and_opens_again() {
    let (server, gitlab) = gitlab(by_iid(mr("opened"))).await;
    gitlab
        .resolve("gid://gitlab/Discussion/abc", true)
        .await
        .expect("resolved");
    gitlab
        .resolve("gid://gitlab/Discussion/abc", false)
        .await
        .expect("opened again");
    let calls: Vec<serde_json::Value> = sent(&server)
        .await
        .into_iter()
        .filter(|one| {
            one["query"]
                .as_str()
                .is_some_and(|q| q.contains("discussionToggleResolve"))
        })
        .map(|one| one["variables"].clone())
        .collect();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0]["resolve"], true);
    assert_eq!(calls[1]["resolve"], false);
}

#[tokio::test]
async fn a_review_posts_its_notes_its_words_and_then_asks_for_changes() {
    let (server, gitlab) = gitlab(by_iid(mr("opened"))).await;
    let notes = [crate::Posted {
        path: "src/lib.rs",
        from: 12,
        to: 14,
        body: "issue: this leaks",
    }];
    gitlab
        .review(
            &repo(),
            "7",
            crate::Verdict {
                said: groove_types::ReviewVerdict::RequestChanges,
                body: "two things to fix",
                notes: &notes,
            },
        )
        .await
        .expect("the review is posted");
    let sent = sent(&server).await;
    assert_eq!(
        variables(&sent, "createLatestDiffNote")["body"],
        "issue: this leaks",
        "the note goes up on its own"
    );
    assert_eq!(
        variables(&sent, "createNote")["body"],
        "two things to fix",
        "the words go up as a comment"
    );
    let asked = variables(&sent, "mergeRequestRequestChanges");
    assert_eq!(asked["path"], "wiremind/devops/overwhelm");
    assert_eq!(asked["iid"], "7", "gitlab counts by iid");
}

#[tokio::test]
async fn an_approval_is_the_one_call_gitlab_keeps_out_of_graphql() {
    let (server, gitlab) = gitlab(by_iid(mr("opened"))).await;
    gitlab
        .review(
            &repo(),
            "7",
            crate::Verdict {
                said: groove_types::ReviewVerdict::Approve,
                body: "",
                notes: &[],
            },
        )
        .await
        .expect("approved");
    let paths: Vec<String> = server
        .received_requests()
        .await
        .unwrap_or_default()
        .iter()
        .map(|one| one.url.path().to_string())
        .collect();
    assert!(
        paths.iter().any(|path| path
            == "/api/v4/projects/wiremind%2Fdevops%2Foverwhelm/merge_requests/7/approve"),
        "{paths:?}"
    );
}

#[tokio::test]
async fn a_comment_goes_on_the_merge_request_itself() {
    let (server, gitlab) = gitlab(by_iid(mr("opened"))).await;
    gitlab
        .comment(&repo(), "7", "note: the pipeline is flaky")
        .await
        .expect("commented");
    let at = variables(&sent(&server).await, "createNote");
    assert_eq!(at["mr"], "gid://gitlab/MergeRequest/99");
    assert_eq!(at["body"], "note: the pipeline is flaky");
}

#[tokio::test]
async fn a_note_on_one_line_names_no_end_at_all() {
    let (server, gitlab) = gitlab(by_iid(mr("opened"))).await;
    gitlab
        .note_on(
            &repo(),
            "7",
            crate::Posted {
                path: "src/lib.rs",
                from: 12,
                to: 12,
                body: "issue: this leaks",
            },
        )
        .await
        .expect("the note is posted");
    let sent = sent(&server).await;
    let at = variables(&sent, "createLatestDiffNote");
    assert_eq!(at["from"].as_u64(), Some(12));
    assert!(
        at["to"].is_null(),
        "gitlab refuses an end that is not past it"
    );
    let query = sent
        .iter()
        .filter_map(|one| one["query"].as_str())
        .find(|one| one.contains("createLatestDiffNote"))
        .expect("the mutation");
    assert!(!query.contains("endNewLine"), "{query}");
}
