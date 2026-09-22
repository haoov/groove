use groove_token::Token;
use groove_types::{CiState, MrState, Repo, RepoId, ReviewState};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::Github;

fn repo() -> Repo {
    Repo {
        id: RepoId::new("r1"),
        host: "github.com".into(),
        group_path: "acme/team".into(),
        project: "groove".into(),
        local_path: "/main/groove".into(),
    }
}

/// One pull request as GitHub answers for it.
fn pr() -> serde_json::Value {
    serde_json::json!({
        "number": 7,
        "title": "fix(forge): read the checks",
        "body": "## What\nthe checks",
        "state": "OPEN",
        "isDraft": false,
        "url": "https://github.com/acme/groove/pull/7",
        "createdAt": "2026-09-18T08:00:00Z",
        "updatedAt": "2026-09-19T09:30:00Z",
        "headRefName": "fix/checks",
        "headRefOid": "cafe1234",
        "id": "PR_node",
        "baseRefName": "main",
        "reviewDecision": "APPROVED",
        "author": { "login": "haoov" },
        "latestReviews": { "nodes": [
            { "state": "APPROVED", "submittedAt": "2026-09-19T09:00:00Z",
              "author": { "login": "reviewer" } },
            { "state": "COMMENTED", "submittedAt": "2026-09-19T09:10:00Z",
              "author": { "login": "passer-by" } }
        ]},
        "reviewRequests": { "nodes": [{ "requestedReviewer": { "login": "awaited" } }] },
        "timelineItems": { "nodes": [
            { "createdAt": "2026-09-18T08:05:00Z", "requestedReviewer": { "login": "awaited" } }
        ]},
        "commits": { "nodes": [{ "commit": { "statusCheckRollup": { "contexts": { "nodes": [
            { "__typename": "CheckRun", "status": "COMPLETED", "conclusion": "SUCCESS",
              "detailsUrl": "https://github.com/acme/groove/runs/1",
              "completedAt": "2026-09-19T09:40:00Z" },
            { "__typename": "CheckRun", "status": "COMPLETED", "conclusion": "FAILURE",
              "detailsUrl": "https://github.com/acme/groove/runs/2",
              "completedAt": "2026-09-19T09:25:00Z" }
        ]}}}}]},
        "reviewThreads": { "nodes": [{
            "id": "THREAD_1",
            "isResolved": false,
            "path": "src/lib.rs",
            "line": 12,
            "diffSide": "RIGHT",
            "comments": { "nodes": [
                { "body": "issue: this drops the error", "createdAt": "2026-09-19T09:00:00Z",
                  "line": 12, "author": { "login": "reviewer" } }
            ]}
        }]},
        "comments": { "nodes": [
            { "id": "COMMENT_1", "body": "note: rebased", "createdAt": "2026-09-19T09:15:00Z",
              "author": { "login": "haoov" } }
        ]}
    })
}

/// A server answering every call with `reply`, and a client on it.
async fn github(reply: serde_json::Value) -> (MockServer, Github) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/graphql"))
        .respond_with(ResponseTemplate::new(200).set_body_json(reply))
        .mount(&server)
        .await;
    let host = format!("http://{}", server.address());
    let github = Github::with_token(&host, Token::Fixed("t".into())).expect("a client");
    (server, github)
}

/// The shape a branch query answers with.
fn by_branch(nodes: Vec<serde_json::Value>) -> serde_json::Value {
    serde_json::json!({ "data": {
        "viewer": { "login": "haoov" },
        "repository": { "pullRequests": { "nodes": nodes } }
    }})
}

#[tokio::test]
async fn the_mr_of_a_branch_comes_back_with_its_fields() {
    let (_server, github) = github(by_branch(vec![pr()])).await;
    let read = github
        .open_mr(&repo(), "fix/checks")
        .await
        .expect("the query answers")
        .expect("one open mr");
    assert_eq!(read.number, "7");
    assert_eq!(read.details.title, "fix(forge): read the checks");
    assert_eq!(read.details.author, "haoov");
    assert_eq!(read.details.source_branch, "fix/checks");
    assert_eq!(read.details.target_branch, "main");
    assert_eq!(read.details.state, MrState::Open);
    assert!(!read.details.draft);
}

#[tokio::test]
async fn a_branch_with_no_open_mr_answers_nothing() {
    let (_server, github) = github(by_branch(vec![])).await;
    let read = github
        .open_mr(&repo(), "fix/checks")
        .await
        .expect("the query answers");
    assert!(read.is_none());
}

#[tokio::test]
async fn one_failed_check_makes_the_whole_run_failed() {
    let (_server, github) = github(by_branch(vec![pr()])).await;
    let read = github.open_mr(&repo(), "b").await.unwrap().unwrap();
    let ci = read.ci.expect("the checks are reported");
    assert_eq!(ci.state, CiState::Failed);
    assert_eq!(
        ci.url, "https://github.com/acme/groove/runs/2",
        "the failure"
    );
    assert_eq!(
        ci.finished_at.map(|at| at.day().to_string()),
        Some("2026-09-19".to_string())
    );
}

#[tokio::test]
async fn an_mr_with_no_check_reports_no_ci() {
    let mut bare = pr();
    bare["commits"]["nodes"][0]["commit"]["statusCheckRollup"] = serde_json::Value::Null;
    let (_server, github) = github(by_branch(vec![bare])).await;
    let read = github.open_mr(&repo(), "b").await.unwrap().unwrap();
    assert!(read.ci.is_none());
}

#[tokio::test]
async fn the_verdicts_given_and_the_one_still_awaited_are_all_reviewers() {
    let (_server, github) = github(by_branch(vec![pr()])).await;
    let read = github.open_mr(&repo(), "b").await.unwrap().unwrap();
    let states: Vec<(&str, ReviewState)> = read
        .details
        .reviewers
        .iter()
        .map(|one| (one.name.as_str(), one.state))
        .collect();
    assert_eq!(
        states,
        vec![
            ("reviewer", ReviewState::Approved),
            ("passer-by", ReviewState::Commented),
            ("awaited", ReviewState::Requested),
        ]
    );
    let asked = read.details.review_requested_at().expect("a request time");
    assert_eq!(
        asked,
        groove_types::Timestamp::parse("2026-09-18T08:05:00Z").unwrap()
    );
}

#[tokio::test]
async fn the_viewer_is_told_apart_from_the_reviewer_who_approved() {
    let (_server, github) = github(by_branch(vec![pr()])).await;
    let read = github.open_mr(&repo(), "b").await.unwrap().unwrap();
    let approval = read.details.approval.expect("an approval");
    assert!(approval.approved);
    assert_eq!(approval.approved_by, vec!["reviewer".to_string()]);
    assert!(
        !approval.approved_by_me,
        "haoov wrote it, reviewer approved"
    );
}

#[tokio::test]
async fn a_thread_keeps_its_line_and_a_comment_keeps_none() {
    let (_server, github) = github(by_branch(vec![pr()])).await;
    let read = github.open_mr(&repo(), "b").await.unwrap().unwrap();
    assert_eq!(read.threads.len(), 2);
    let anchored = &read.threads[0];
    assert_eq!(anchored.id, "THREAD_1");
    let note = &anchored.notes[0];
    assert!(note.resolvable);
    assert!(!note.resolved);
    let at = note.position.clone().expect("a position");
    assert_eq!(at.new_path.as_deref(), Some("src/lib.rs"));
    assert_eq!(at.new_line, Some(12));
    let loose = &read.threads[1];
    assert_eq!(loose.id, "COMMENT_1");
    assert!(!loose.notes[0].resolvable);
    assert!(loose.notes[0].position.is_none());
}

#[tokio::test]
async fn a_note_on_the_old_side_is_anchored_nowhere() {
    let mut left = pr();
    left["reviewThreads"]["nodes"][0]["diffSide"] = serde_json::json!("LEFT");
    let (_server, github) = github(by_branch(vec![left])).await;
    let read = github.open_mr(&repo(), "b").await.unwrap().unwrap();
    assert!(read.threads[0].notes[0].position.is_none());
}

#[tokio::test]
async fn an_mr_read_by_number_answers_the_same_way() {
    let reply = serde_json::json!({ "data": {
        "viewer": { "login": "haoov" },
        "repository": { "pullRequest": pr() }
    }});
    let (_server, github) = github(reply).await;
    let read = github.read_mr(&repo(), "7").await.expect("the mr is read");
    assert_eq!(
        read.details.web_url,
        "https://github.com/acme/groove/pull/7"
    );
}

#[tokio::test]
async fn a_number_the_host_does_not_know_is_an_error() {
    let reply = serde_json::json!({ "data": {
        "viewer": { "login": "haoov" },
        "repository": { "pullRequest": null }
    }});
    let (_server, github) = github(reply).await;
    let missing = github.read_mr(&repo(), "9").await.expect_err("no such mr");
    assert!(
        missing.to_string().contains("acme/team/groove"),
        "{missing}"
    );
}

#[tokio::test]
async fn what_the_host_says_is_wrong_with_the_query_becomes_the_error() {
    let reply = serde_json::json!({ "errors": [{ "message": "Bad credentials" }] });
    let (_server, github) = github(reply).await;
    let refused = github
        .open_mr(&repo(), "b")
        .await
        .expect_err("the query is refused");
    assert!(refused.to_string().contains("Bad credentials"), "{refused}");
}

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
    let reply = serde_json::json!({ "data": { "search": { "nodes": [
        asked(7, "2026-09-18T08:00:00Z", serde_json::Value::Null),
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
}

/// One PR as a read by its number answers, and the mutations after it.
fn by_number(one: serde_json::Value) -> serde_json::Value {
    serde_json::json!({ "data": {
        "viewer": { "login": "haoov" },
        "repository": { "pullRequest": one },
        "addPullRequestReviewThread": { "thread": { "id": "PRRT_1" } },
        "addPullRequestReview": { "pullRequestReview": { "id": "PRR_1" } },
        "addComment": { "clientMutationId": null },
        "addPullRequestReviewThreadReply": { "comment": { "id": "PRRC_1" } },
        "resolveReviewThread": { "thread": { "id": "PRRT_1" } },
        "unresolveReviewThread": { "thread": { "id": "PRRT_1" } }
    }})
}

async fn sent(server: &MockServer) -> Vec<serde_json::Value> {
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
async fn a_note_opens_a_thread_on_the_lines_it_covers() {
    let (server, github) = github(by_number(pr())).await;
    github
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
        .expect("the thread is opened");
    let at = variables(&sent(&server).await, "addPullRequestReviewThread");
    assert_eq!(at["mr"], "PR_node");
    assert_eq!(at["path"], "src/lib.rs");
    assert_eq!(
        (at["from"].as_u64(), at["to"].as_u64()),
        (Some(12), Some(14)),
        "the range reads from its first line to its last"
    );
    assert_eq!(at["body"], "issue: this leaks");
}

#[tokio::test]
async fn a_reply_goes_under_the_thread_it_answers() {
    let (server, github) = github(by_number(pr())).await;
    github.reply_to("PRRT_1", "fixed").await.expect("replied");
    let at = variables(&sent(&server).await, "addPullRequestReviewThreadReply");
    assert_eq!(at["thread"], "PRRT_1");
    assert_eq!(at["body"], "fixed");
}

#[tokio::test]
async fn a_thread_resolves_and_opens_again() {
    let (server, github) = github(by_number(pr())).await;
    github.resolve("PRRT_1", true).await.expect("resolved");
    github.resolve("PRRT_1", false).await.expect("opened again");
    let asked: Vec<String> = sent(&server)
        .await
        .into_iter()
        .filter_map(|one| one["query"].as_str().map(str::to_string))
        .collect();
    assert!(
        asked.iter().any(|q| q.contains("resolveReviewThread(")),
        "{asked:?}"
    );
    assert!(
        asked.iter().any(|q| q.contains("unresolveReviewThread(")),
        "{asked:?}"
    );
}

#[tokio::test]
async fn a_review_carries_its_verdict_its_words_and_its_notes() {
    let (server, github) = github(by_number(pr())).await;
    let notes = [crate::Posted {
        path: "src/lib.rs",
        from: 12,
        to: 14,
        body: "issue: this leaks",
    }];
    github
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
    let at = variables(&sent(&server).await, "addPullRequestReview(");
    assert_eq!(at["mr"], "PR_node");
    assert_eq!(at["event"], "REQUEST_CHANGES");
    assert_eq!(at["body"], "two things to fix");
    let threads = at["threads"].as_array().expect("its notes");
    assert_eq!(threads.len(), 1, "one call carries all of it");
    assert_eq!(threads[0]["path"], "src/lib.rs");
    assert_eq!(
        (
            threads[0]["startLine"].as_u64(),
            threads[0]["line"].as_u64()
        ),
        (Some(12), Some(14))
    );
}

#[tokio::test]
async fn an_approval_says_so_in_githubs_own_word() {
    let (server, github) = github(by_number(pr())).await;
    github
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
    let at = variables(&sent(&server).await, "addPullRequestReview(");
    assert_eq!(at["event"], "APPROVE");
}

#[tokio::test]
async fn a_comment_goes_on_the_pull_request_itself() {
    let (server, github) = github(by_number(pr())).await;
    github
        .comment(&repo(), "7", "note: the ci is flaky")
        .await
        .expect("commented");
    let at = variables(&sent(&server).await, "addComment");
    assert_eq!(at["mr"], "PR_node");
    assert_eq!(at["body"], "note: the ci is flaky");
}
