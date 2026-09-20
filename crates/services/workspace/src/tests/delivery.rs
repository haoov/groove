use groove_forge::{Github, Remote, Token};
use groove_types::{MrState, Repo, RepoId, SessionId, Timestamp, Worktree, WorktreeId};
use wiremock::matchers::{body_string_contains, method};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::{Delivery, Service};

fn repo(host: &str) -> Repo {
    Repo {
        id: RepoId::new("r1"),
        host: host.to_string(),
        group_path: "acme".into(),
        project: "groove".into(),
        local_path: "/main/groove".into(),
    }
}

fn worktree() -> Worktree {
    Worktree {
        id: WorktreeId::new("w1"),
        session: SessionId::new("s1"),
        repo: RepoId::new("r1"),
        branch: "fix/one".into(),
        path: "/w/one".into(),
        base_ref: Some("main".into()),
        created_at: Timestamp::new(0),
    }
}

/// One pull request, in whichever state the test wants it.
fn pr(state: &str) -> serde_json::Value {
    serde_json::json!({
        "number": 7,
        "title": "fix: one",
        "body": "",
        "state": state,
        "isDraft": false,
        "url": "https://github.com/acme/groove/pull/7",
        "createdAt": "2026-09-18T08:00:00Z",
        "updatedAt": "2026-09-19T09:30:00Z",
        "headRefName": "fix/one",
        "baseRefName": "main",
        "author": { "login": "haoov" }
    })
}

/// A service on its own database, with the rows an MR hangs off already seeded.
async fn service() -> Service {
    let service = Service::in_memory().await.expect("a database");
    for statement in [
        "INSERT INTO sessions (id, kind, title, created_at)
         VALUES ('s1', 'explorer', 'work', 0)",
        "INSERT INTO repos (id, host, group_path, project, local_path)
         VALUES ('r1', 'github.com', 'acme', 'groove', '/main/groove')",
        "INSERT INTO worktrees (id, session_id, repo_id, branch, path, created_at)
         VALUES ('w1', 's1', 'r1', 'fix/one', '/w/one', 0)",
    ] {
        sqlx::query(statement)
            .execute(service.store().db().pool())
            .await
            .expect("the row is seeded");
    }
    service
}

/// A host answering the branch query with `by_branch` and the number query with
/// `by_number`.
async fn host(by_branch: serde_json::Value, by_number: serde_json::Value) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(body_string_contains("pullRequests("))
        .respond_with(ResponseTemplate::new(200).set_body_json(by_branch))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(body_string_contains("pullRequest(number"))
        .respond_with(ResponseTemplate::new(200).set_body_json(by_number))
        .mount(&server)
        .await;
    server
}

fn answer(key: &str, pr: serde_json::Value) -> serde_json::Value {
    serde_json::json!({ "data": {
        "viewer": { "login": "haoov" },
        "repository": { key: pr }
    }})
}

/// A repo on the mock host, and a GitHub client named outright: the host rule cannot
/// read a forge out of an address.
fn remote(server: &MockServer) -> (Repo, Remote) {
    let host = format!("http://{}", server.address());
    let client = Github::with_token(&host, Token::Fixed("t".into())).expect("a client");
    (repo(&host), Remote::Github(client))
}

#[tokio::test]
async fn a_worktree_with_no_mr_asks_the_branch_and_writes_the_row() {
    let server = host(
        answer("pullRequests", serde_json::json!({ "nodes": [pr("OPEN")] })),
        serde_json::Value::Null,
    )
    .await;
    let (repo, remote) = remote(&server);
    let service = service().await;
    let read = service
        .read(&remote, &repo, &worktree())
        .await
        .expect("the forge answers")
        .expect("one mr");
    assert_eq!(read.mr.remote_id, "7");
    assert_eq!(read.mr.state, MrState::Open);
    let stored = service.stored(&worktree().id).await.unwrap();
    assert_eq!(stored.map(|mr| mr.remote_id), Some("7".to_string()));
}

#[tokio::test]
async fn once_the_number_is_known_the_read_follows_it_and_sees_the_merge() {
    let server = host(
        answer("pullRequests", serde_json::json!({ "nodes": [pr("OPEN")] })),
        answer("pullRequest", pr("MERGED")),
    )
    .await;
    let (repo, remote) = remote(&server);
    let service = service().await;
    service
        .read(&remote, &repo, &worktree())
        .await
        .unwrap()
        .expect("found by branch");
    let again = service
        .read(&remote, &repo, &worktree())
        .await
        .unwrap()
        .expect("found by number");
    assert_eq!(again.mr.state, MrState::Merged, "the number query answered");
}

#[tokio::test]
async fn a_branch_the_forge_has_no_mr_for_leaves_no_row() {
    let server = host(
        answer("pullRequests", serde_json::json!({ "nodes": [] })),
        serde_json::Value::Null,
    )
    .await;
    let (repo, remote) = remote(&server);
    let service = service().await;
    let read = service.read(&remote, &repo, &worktree()).await.unwrap();
    assert!(read.is_none());
    assert!(service.stored(&worktree().id).await.unwrap().is_none());
}

#[tokio::test]
async fn an_mr_the_user_closed_is_forgotten() {
    let server = host(
        answer("pullRequests", serde_json::json!({ "nodes": [pr("OPEN")] })),
        serde_json::Value::Null,
    )
    .await;
    let (repo, remote) = remote(&server);
    let service = service().await;
    service.read(&remote, &repo, &worktree()).await.unwrap();
    service.forget(&worktree().id).await.unwrap();
    assert!(service.stored(&worktree().id).await.unwrap().is_none());
}

#[test]
fn a_failed_read_ages_what_stands_and_a_missing_mr_clears_it() {
    let mut delivery = Delivery::default();
    assert!(!delivery.stale);
    delivery.aged();
    assert!(delivery.stale, "nothing read, and it says so");
    delivery.none();
    assert!(!delivery.stale);
    assert!(delivery.mr.is_none());
    assert!(delivery.read.is_none());
}
