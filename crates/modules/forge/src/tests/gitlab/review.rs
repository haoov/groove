//! GitLab review writes: notes, replies, resolutions, reviews and comments.

use super::*;

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
    assert_eq!(asked["path"], "acme/devops/orchard");
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
            == "/api/v4/projects/acme%2Fdevops%2Forchard/merge_requests/7/approve"),
        "{paths:?}"
    );
}

#[tokio::test]
async fn a_review_that_only_comments_marks_the_viewer_as_having_reviewed() {
    let (server, gitlab) = gitlab(by_iid(mr("opened"))).await;
    let said = crate::Verdict {
        said: groove_types::ReviewVerdict::Comment,
        body: "looks fine",
        notes: &[],
    };
    gitlab.review(&repo(), "7", said).await.expect("reviewed");
    let asked = server.received_requests().await.unwrap_or_default();
    let publish = asked
        .iter()
        .find(|one| {
            one.url
                .path()
                .ends_with("/merge_requests/7/draft_notes/bulk_publish")
        })
        .expect("the review is published");
    assert_eq!(publish.method.as_str(), "POST");
    assert_eq!(publish.url.query(), Some("reviewer_state=reviewed"));
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
