//! What a verdict carries, and what a call that fails halfway leaves behind.

use groove_annotations::New;
use groove_mrs::Stored as _;
use groove_types::{Annotation, MrState, RepoId, ReviewVerdict, SessionId};
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

/// Two notes of this session, on two lines of one file.
async fn two_notes(service: &Service) -> Vec<Annotation> {
    let mut made = Vec::new();
    for line in [10, 20] {
        let new = New {
            session: SessionId::new("s1"),
            repo: RepoId::new("r1"),
            file_path: "src/lib.rs".into(),
            start_line: line,
            end_line: line,
            content: format!("issue: line {line}"),
            author: "you".into(),
        };
        made.push(
            service
                .create_note(new, groove_types::Timestamp::new(1))
                .await
                .expect("a note"),
        );
    }
    made
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

/// Which notes of the session stand open.
async fn open_notes(service: &Service) -> Vec<String> {
    service
        .notes(&SessionId::new("s1"))
        .await
        .expect("the notes")
        .into_iter()
        .filter(|note| note.status == groove_types::AnnotationStatus::Open)
        .map(|note| note.content)
        .collect()
}

#[tokio::test]
async fn a_note_that_went_up_is_gone_even_when_the_next_one_fails() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(body_string_contains("line 20"))
        .respond_with(ResponseTemplate::new(500).set_body_string("no"))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mr()))
        .mount(&server)
        .await;
    let service = service().await;
    held(&service).await;
    let notes = two_notes(&service).await;
    let (repo, service) = gitlab(&server, service);

    let refused = service
        .review(
            &repo,
            &worktree().id,
            Said {
                verdict: ReviewVerdict::Approve,
                body: "",
                notes: &notes,
            },
        )
        .await
        .expect_err("the second note is refused");
    assert!(format!("{refused}").contains("500"), "{refused}");
    let left: Vec<String> = service
        .notes(&SessionId::new("s1"))
        .await
        .expect("the notes")
        .into_iter()
        .map(|note| note.content)
        .collect();
    assert_eq!(
        left,
        ["issue: line 20"],
        "the first is up and gone; only the second is left to post"
    );
}

#[tokio::test]
async fn a_verdict_resolves_every_note_it_carried() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mr()))
        .mount(&server)
        .await;
    let service = service().await;
    held(&service).await;
    let notes = two_notes(&service).await;
    let (repo, service) = gitlab(&server, service);

    service
        .review(
            &repo,
            &worktree().id,
            Said {
                verdict: ReviewVerdict::Approve,
                body: "looks right",
                notes: &notes,
            },
        )
        .await
        .expect("the review is posted");
    assert!(open_notes(&service).await.is_empty(), "every one is up");
}

#[tokio::test]
async fn a_note_of_another_repo_is_refused() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mr()))
        .mount(&server)
        .await;
    let service = service().await;
    held(&service).await;
    let mut note = two_notes(&service).await.remove(0);
    note.repo = RepoId::new("another");
    let (repo, service) = gitlab(&server, service);

    let refused = service
        .post_note(&repo, &worktree().id, &note)
        .await
        .expect_err("it is not this repo's note");
    assert!(format!("{refused}").contains("not of r1"), "{refused}");
    assert_eq!(MrState::Open, MrState::Open, "nothing about the mr changed");
}

#[tokio::test]
async fn posting_one_note_keeps_the_others() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mr()))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mr()))
        .mount(&server)
        .await;
    let service = service().await;
    held(&service).await;
    let note = two_notes(&service).await.remove(0);
    let (repo, service) = gitlab(&server, service);
    service
        .post_note(&repo, &worktree().id, &note)
        .await
        .expect("the note is posted");
    assert_eq!(open_notes(&service).await, ["issue: line 20"]);
}
