//! An MR opened on either forge is assigned to the one who opened it.

use super::{github as hub, gitlab as lab};
use crate::Proposed;

fn proposed() -> Proposed<'static> {
    Proposed {
        head: "fix/pipeline",
        base: Some("main"),
        title: "fix: one",
        body: "why",
    }
}

#[tokio::test]
async fn a_gitlab_mr_opened_is_assigned_to_its_author() {
    let reply = serde_json::json!({ "data": {
        "currentUser": { "username": "rsabbah" },
        "mergeRequestCreate": { "errors": [], "mergeRequest": lab::mr("opened") },
        "mergeRequestSetAssignees": { "errors": [] }
    }});
    let (server, gitlab) = lab::gitlab(reply).await;
    let opened = gitlab
        .create_mr(&lab::repo(), proposed())
        .await
        .expect("it is opened");
    gitlab
        .assign(&lab::repo(), &opened)
        .await
        .expect("it is assigned");
    let sent = lab::sent(&server).await;
    let assigned = sent
        .iter()
        .find(|one| {
            one["query"]
                .as_str()
                .is_some_and(|q| q.contains("SetAssignees"))
        })
        .expect("the MR is assigned");
    assert_eq!(assigned["variables"]["who"], serde_json::json!(["rsabbah"]));
    assert_eq!(assigned["variables"]["iid"], "7");
}

#[tokio::test]
async fn a_github_pr_opened_is_assigned_to_the_viewer() {
    let reply = serde_json::json!({ "data": {
        "viewer": { "id": "U_me", "login": "haoov" },
        "repository": { "id": "R_1", "defaultBranchRef": { "name": "main" } },
        "createPullRequest": { "pullRequest": hub::pr() },
        "addAssigneesToAssignable": { "clientMutationId": null }
    }});
    let (server, github) = hub::github(reply).await;
    let opened = github
        .create_mr(&hub::repo(), proposed())
        .await
        .expect("it is opened");
    github.assign(&opened).await.expect("it is assigned");
    let sent = hub::sent(&server).await;
    let assigned = hub::variables(&sent, "addAssigneesToAssignable");
    assert_eq!(assigned["mr"], "PR_node");
    assert_eq!(assigned["who"], serde_json::json!(["U_me"]));
    let mutations = sent
        .iter()
        .filter_map(|one| one["query"].as_str())
        .filter(|q| q.starts_with("mutation"));
    for one in mutations {
        let asks_viewer = one
            .match_indices("viewer {")
            .any(|(at, _)| !one[..at].ends_with(|c: char| c.is_alphabetic()));
        assert!(!asks_viewer, "GitHub refuses it there: {one}");
    }
}
