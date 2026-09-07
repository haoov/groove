//! GitHub over REST and GraphQL (see `api`); `gh` only supplies the token.
//! Review threads, CI and approval use GraphQL; plain CRUD uses REST.

use reqwest::Method;

use crate::core::db::models::Repo;

use super::client::{
    CiStatus, MrApproval, MrDetails, MrNote, MrThread, NotePosition, PlatformClient,
};
use crate::core::forge::api;

/// GitHub's `owner`: the last segment of the group path.
fn owner_of(repo: &Repo) -> &str {
    repo.group_path
        .rsplit('/')
        .next()
        .unwrap_or(&repo.group_path)
}

fn pr_path(repo: &Repo, remote_id: &str) -> String {
    format!(
        "repos/{}/{}/pulls/{remote_id}",
        owner_of(repo),
        repo.project
    )
}

/// GraphQL variables addressing one PR.
fn pr_vars(repo: &Repo, remote_id: &str) -> serde_json::Value {
    serde_json::json!({
        "owner": owner_of(repo),
        "name": repo.project,
        "number": remote_id.parse::<i64>().unwrap_or(0),
    })
}

/// The current login, cached for the run.
static GH_LOGIN: std::sync::OnceLock<String> = std::sync::OnceLock::new();

async fn gh_login(host: &str) -> Option<String> {
    if let Some(l) = GH_LOGIN.get() {
        return Some(l.clone());
    }
    let v = api::github(host, Method::GET, "user", None).await.ok()?;
    let login = v["login"].as_str()?.to_string();
    let _ = GH_LOGIN.set(login.clone());
    Some(login)
}

/// One status word for a rollup of checks; the worst state wins.
fn rollup_status(rollup: &serde_json::Value) -> Option<(String, String)> {
    let nodes = rollup.as_array()?;
    if nodes.is_empty() {
        return None;
    }
    let mut running = false;
    let mut failed = false;
    let mut url = String::new();
    for n in nodes {
        // A check run reports `status` + `conclusion`; a commit status reports `state`.
        let status = n["status"].as_str().unwrap_or("");
        let conclusion = n["conclusion"].as_str().unwrap_or("");
        let state = n["state"].as_str().unwrap_or("");
        if url.is_empty() {
            if let Some(u) = n["detailsUrl"].as_str().or(n["targetUrl"].as_str()) {
                if !u.is_empty() {
                    url = u.to_string();
                }
            }
        }
        match (status, conclusion, state) {
            (
                _,
                "FAILURE" | "TIMED_OUT" | "CANCELLED" | "ACTION_REQUIRED" | "STARTUP_FAILURE",
                _,
            )
            | (_, _, "FAILURE" | "ERROR") => failed = true,
            ("QUEUED" | "IN_PROGRESS" | "PENDING" | "WAITING" | "REQUESTED", _, _)
            | (_, _, "PENDING") => running = true,
            _ => {}
        }
    }
    // Keep GitLab's vocabulary: the UI's `ciGroup` maps these.
    let status = if failed {
        "failed"
    } else if running {
        "running"
    } else {
        "success"
    };
    Some((status.to_string(), url))
}

/// GraphQL review threads and conversation comments → the UI's threads.
fn threads_from_graphql(pr: &serde_json::Value) -> Vec<MrThread> {
    let mut threads = vec![];
    for t in pr["reviewThreads"]["nodes"]
        .as_array()
        .cloned()
        .unwrap_or_default()
    {
        let resolved = t["isResolved"].as_bool().unwrap_or(false);
        // The UI anchors new-side lines only; a LEFT-side note keeps no position.
        let on_new_side = t["diffSide"].as_str().unwrap_or("RIGHT") == "RIGHT";
        let path = t["path"].as_str().unwrap_or("").to_string();
        let notes: Vec<MrNote> = t["comments"]["nodes"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .map(|c| {
                let line = c["line"].as_i64().or(c["originalLine"].as_i64());
                MrNote {
                    author: c["author"]["login"].as_str().unwrap_or("").to_string(),
                    body: c["body"].as_str().unwrap_or("").to_string(),
                    created_at: c["createdAt"].as_str().unwrap_or("").to_string(),
                    resolved,
                    resolvable: true,
                    position: line.filter(|_| on_new_side).map(|line| NotePosition {
                        new_path: Some(path.clone()),
                        new_line: Some(line),
                        old_path: None,
                        old_line: None,
                        end_new_line: None,
                    }),
                }
            })
            .collect();
        if notes.is_empty() {
            continue;
        }
        threads.push(MrThread {
            id: t["id"].as_str().unwrap_or("").to_string(),
            notes,
        });
    }

    // Conversation comments: no position, not resolvable.
    for c in pr["comments"]["nodes"]
        .as_array()
        .cloned()
        .unwrap_or_default()
    {
        threads.push(MrThread {
            id: c["databaseId"].to_string(),
            notes: vec![MrNote {
                author: c["author"]["login"].as_str().unwrap_or("").to_string(),
                body: c["body"].as_str().unwrap_or("").to_string(),
                created_at: c["createdAt"].as_str().unwrap_or("").to_string(),
                resolved: true,
                resolvable: false,
                position: None,
            }],
        });
    }
    threads
}

pub(super) struct GhClient;

#[async_trait::async_trait]
impl PlatformClient for GhClient {
    fn platform_name(&self) -> &'static str {
        "github"
    }

    async fn create_mr(
        &self,
        repo: &Repo,
        branch: &str,
        target: &str,
        title: &str,
        description: &str,
    ) -> anyhow::Result<(String, String)> {
        let v = api::github(
            &repo.host,
            Method::POST,
            &format!("repos/{}/{}/pulls", owner_of(repo), repo.project),
            Some(&serde_json::json!({
                "title": title,
                "body": description,
                "head": branch,
                "base": target,
            })),
        )
        .await?;
        let number = v["number"]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("created PR has no number: {v}"))?
            .to_string();
        let url = v["html_url"].as_str().unwrap_or("").to_string();

        // Self-assignment is best-effort.
        if let Some(login) = gh_login(&repo.host).await {
            let _ = api::github(
                &repo.host,
                Method::POST,
                &format!(
                    "repos/{}/{}/issues/{number}/assignees",
                    owner_of(repo),
                    repo.project
                ),
                Some(&serde_json::json!({ "assignees": [login] })),
            )
            .await;
        }
        Ok((number, url))
    }

    async fn update_mr(
        &self,
        repo: &Repo,
        remote_id: &str,
        title: Option<&str>,
        description: Option<&str>,
    ) -> anyhow::Result<()> {
        let mut body = serde_json::Map::new();
        if let Some(t) = title {
            body.insert("title".into(), serde_json::json!(t));
        }
        if let Some(d) = description {
            body.insert("body".into(), serde_json::json!(d));
        }
        api::github(
            &repo.host,
            Method::PATCH,
            &pr_path(repo, remote_id),
            Some(&serde_json::Value::Object(body)),
        )
        .await?;
        Ok(())
    }

    async fn close_mr(&self, repo: &Repo, remote_id: &str) -> anyhow::Result<()> {
        api::github(
            &repo.host,
            Method::PATCH,
            &pr_path(repo, remote_id),
            Some(&serde_json::json!({ "state": "closed" })),
        )
        .await?;
        Ok(())
    }

    async fn get_mr_details(&self, repo: &Repo, remote_id: &str) -> anyhow::Result<MrDetails> {
        let v = api::github(&repo.host, Method::GET, &pr_path(repo, remote_id), None).await?;
        // REST has no "merged" state; it is "closed" with a merged flag.
        let state = if v["merged"].as_bool() == Some(true) {
            "merged".to_string()
        } else {
            v["state"].as_str().unwrap_or("open").to_lowercase()
        };
        Ok(MrDetails {
            title: v["title"].as_str().unwrap_or("").to_string(),
            description: v["body"].as_str().unwrap_or("").to_string(),
            author: v["user"]["login"].as_str().unwrap_or("").to_string(),
            source_branch: v["head"]["ref"].as_str().unwrap_or("").to_string(),
            target_branch: v["base"]["ref"].as_str().unwrap_or("").to_string(),
            state,
            draft: v["draft"].as_bool().unwrap_or(false),
            created_at: v["created_at"].as_str().unwrap_or("").to_string(),
            web_url: v["html_url"].as_str().unwrap_or("").to_string(),
            approval: None,
        })
    }

    async fn get_mr_ci(&self, repo: &Repo, remote_id: &str) -> anyhow::Result<Option<CiStatus>> {
        const QUERY: &str = r#"
query($owner:String!, $name:String!, $number:Int!) {
  repository(owner:$owner, name:$name) {
    pullRequest(number:$number) {
      url
      commits(last: 1) { nodes { commit { statusCheckRollup { contexts(first: 100) { nodes {
        __typename
        ... on CheckRun { status conclusion detailsUrl }
        ... on StatusContext { state targetUrl }
      }}}}}}
    }
  }
}"#;
        let v = api::github_graphql(&repo.host, QUERY, pr_vars(repo, remote_id)).await?;
        let pr = &v["data"]["repository"]["pullRequest"];
        let contexts =
            &pr["commits"]["nodes"][0]["commit"]["statusCheckRollup"]["contexts"]["nodes"];
        let Some((status, url)) = rollup_status(contexts) else {
            return Ok(None);
        };
        Ok(Some(CiStatus {
            url: if url.is_empty() {
                pr["url"].as_str().unwrap_or("").to_string()
            } else {
                url
            },
            status,
        }))
    }

    async fn get_mr_threads(&self, repo: &Repo, remote_id: &str) -> anyhow::Result<Vec<MrThread>> {
        const QUERY: &str = r#"
query($owner:String!, $name:String!, $number:Int!) {
  repository(owner:$owner, name:$name) {
    pullRequest(number:$number) {
      reviewThreads(first: 100) {
        nodes {
          id isResolved isOutdated path line diffSide
          comments(first: 100) {
            nodes { databaseId body path line originalLine createdAt author { login } }
          }
        }
      }
      comments(first: 100) {
        nodes { databaseId body createdAt author { login } }
      }
    }
  }
}"#;
        let v = api::github_graphql(&repo.host, QUERY, pr_vars(repo, remote_id)).await?;
        Ok(threads_from_graphql(
            &v["data"]["repository"]["pullRequest"],
        ))
    }

    async fn reply_to_thread(
        &self,
        repo: &Repo,
        _remote_id: &str,
        thread_id: &str,
        body: &str,
    ) -> anyhow::Result<()> {
        const MUTATION: &str = r#"
mutation($threadId:ID!, $body:String!) {
  addPullRequestReviewThreadReply(input:{pullRequestReviewThreadId:$threadId, body:$body}) {
    comment { databaseId }
  }
}"#;
        api::github_graphql(
            &repo.host,
            MUTATION,
            serde_json::json!({ "threadId": thread_id, "body": body }),
        )
        .await?;
        Ok(())
    }

    async fn resolve_mr_thread(
        &self,
        repo: &Repo,
        _remote_id: &str,
        thread_id: &str,
    ) -> anyhow::Result<()> {
        const MUTATION: &str = r#"
mutation($threadId:ID!) {
  resolveReviewThread(input:{threadId:$threadId}) { thread { isResolved } }
}"#;
        let v = api::github_graphql(
            &repo.host,
            MUTATION,
            serde_json::json!({ "threadId": thread_id }),
        )
        .await?;
        if v["data"]["resolveReviewThread"]["thread"]["isResolved"].as_bool() == Some(true) {
            return Ok(());
        }
        Err(anyhow::anyhow!(
            "GitHub did not report the thread as resolved"
        ))
    }

    async fn approve_mr(&self, repo: &Repo, remote_id: &str) -> anyhow::Result<()> {
        api::github(
            &repo.host,
            Method::POST,
            &format!("{}/reviews", pr_path(repo, remote_id)),
            Some(&serde_json::json!({ "event": "APPROVE" })),
        )
        .await?;
        Ok(())
    }

    async fn get_mr_approval(&self, repo: &Repo, remote_id: &str) -> anyhow::Result<MrApproval> {
        const QUERY: &str = r#"
query($owner:String!, $name:String!, $number:Int!) {
  repository(owner:$owner, name:$name) {
    pullRequest(number:$number) {
      reviewDecision
      reviews(last: 100) { nodes { state author { login } } }
    }
  }
}"#;
        let v = api::github_graphql(&repo.host, QUERY, pr_vars(repo, remote_id)).await?;
        let pr = &v["data"]["repository"]["pullRequest"];
        // The latest review per author wins.
        let mut latest: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        for r in pr["reviews"]["nodes"]
            .as_array()
            .cloned()
            .unwrap_or_default()
        {
            let Some(login) = r["author"]["login"].as_str() else {
                continue;
            };
            let state = r["state"].as_str().unwrap_or("");
            // COMMENTED does not change an earlier verdict.
            if state == "COMMENTED" || state == "PENDING" {
                continue;
            }
            latest.insert(login.to_string(), state.to_string());
        }
        let by: Vec<String> = latest
            .iter()
            .filter(|(_, s)| *s == "APPROVED")
            .map(|(l, _)| l.clone())
            .collect();
        let me = gh_login(&repo.host).await;
        Ok(MrApproval {
            approved: pr["reviewDecision"].as_str() == Some("APPROVED") || !by.is_empty(),
            approved_by_me: me.map(|m| by.contains(&m)).unwrap_or(false),
            approved_by: by,
        })
    }

    async fn post_mr_comment(
        &self,
        repo: &Repo,
        remote_id: &str,
        body: &str,
        position: Option<(&str, i64)>,
    ) -> anyhow::Result<()> {
        let Some((path, line)) = position else {
            api::github(
                &repo.host,
                Method::POST,
                &format!(
                    "repos/{}/{}/issues/{remote_id}/comments",
                    owner_of(repo),
                    repo.project
                ),
                Some(&serde_json::json!({ "body": body })),
            )
            .await?;
            return Ok(());
        };

        // A positioned review comment needs the remote PR head sha.
        let pr = api::github(&repo.host, Method::GET, &pr_path(repo, remote_id), None).await?;
        let head_sha = pr["head"]["sha"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("PR #{remote_id} has no head sha"))?;

        api::github(
            &repo.host,
            Method::POST,
            &format!("{}/comments", pr_path(repo, remote_id)),
            Some(&serde_json::json!({
                "body": body,
                "commit_id": head_sha,
                "path": path,
                "line": line,
                "side": "RIGHT",
            })),
        )
        .await?;
        Ok(())
    }
}

/// Open PRs where the current user is a requested reviewer, for the review queue.
pub(super) async fn review_requested_prs(host: &str) -> anyhow::Result<Vec<serde_json::Value>> {
    const QUERY: &str = r#"
query {
  search(query: "is:pr is:open review-requested:@me", type: ISSUE, first: 50) {
    nodes {
      ... on PullRequest {
        number title url updatedAt isDraft
        author { login }
        headRefName baseRefName reviewDecision
        repository { nameWithOwner }
      }
    }
  }
}"#;
    let v = api::github_graphql(host, QUERY, serde_json::json!({})).await?;
    Ok(v["data"]["search"]["nodes"]
        .as_array()
        .cloned()
        .unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo(group: &str, project: &str) -> Repo {
        Repo {
            id: "r1".into(),
            host: "github.com".into(),
            group_path: group.into(),
            project: project.into(),
            local_path: "/tmp/x".into(),
        }
    }

    #[test]
    fn owner_is_the_last_group_segment() {
        assert_eq!(owner_of(&repo("cli", "cli")), "cli");
        assert_eq!(owner_of(&repo("acme/team", "svc")), "team");
    }

    #[test]
    fn pr_vars_carry_a_numeric_number() {
        let v = pr_vars(&repo("cli", "cli"), "9000");
        assert_eq!(v["number"], 9000);
        assert_eq!(v["owner"], "cli");
    }

    #[test]
    fn no_checks_means_no_ci_row() {
        assert!(rollup_status(&serde_json::json!([])).is_none());
        assert!(rollup_status(&serde_json::Value::Null).is_none());
    }

    #[test]
    fn all_green_is_success() {
        let r = serde_json::json!([
            { "status": "COMPLETED", "conclusion": "SUCCESS", "detailsUrl": "https://ci/1" },
            { "status": "COMPLETED", "conclusion": "SKIPPED" },
        ]);
        let (status, url) = rollup_status(&r).unwrap();
        assert_eq!(status, "success");
        assert_eq!(url, "https://ci/1");
    }

    #[test]
    fn one_failure_fails_the_rollup() {
        let r = serde_json::json!([
            { "status": "COMPLETED", "conclusion": "SUCCESS" },
            { "status": "COMPLETED", "conclusion": "FAILURE" },
            { "status": "IN_PROGRESS", "conclusion": "" },
        ]);
        assert_eq!(rollup_status(&r).unwrap().0, "failed");
    }

    #[test]
    fn anything_pending_is_running() {
        let r = serde_json::json!([
            { "status": "COMPLETED", "conclusion": "SUCCESS" },
            { "status": "QUEUED", "conclusion": "" },
        ]);
        assert_eq!(rollup_status(&r).unwrap().0, "running");
    }

    #[test]
    fn reads_legacy_commit_statuses_too() {
        let failed = serde_json::json!([{ "state": "FAILURE", "targetUrl": "https://ci/2" }]);
        assert_eq!(
            rollup_status(&failed).unwrap(),
            ("failed".into(), "https://ci/2".into())
        );
        let pending = serde_json::json!([{ "state": "PENDING" }]);
        assert_eq!(rollup_status(&pending).unwrap().0, "running");
        let ok = serde_json::json!([{ "state": "SUCCESS" }]);
        assert_eq!(rollup_status(&ok).unwrap().0, "success");
    }

    /// Captured from cli/cli#9000: a resolved thread, an open one, and a conversation comment.
    const THREADS: &str = r#"{
      "reviewThreads": { "nodes": [
        { "id": "PRRT_kwDODKw3uc48Rk4m", "isResolved": true, "isOutdated": false,
          "path": "pkg/cmd/attestation/verify/verify.go", "line": 130, "diffSide": "RIGHT",
          "comments": { "nodes": [
            { "databaseId": 1583153997, "body": "Could I interest you in the fo",
              "path": "pkg/cmd/attestation/verify/verify.go", "line": 130, "originalLine": 130,
              "createdAt": "2024-04-29T14:06:54Z", "author": { "login": "williammartin" } }
          ] } },
        { "id": "PRRT_kwDODKw3uc48RxjH", "isResolved": false, "isOutdated": false,
          "path": "pkg/cmdutil/auth_check_test.go", "line": 113, "diffSide": "RIGHT",
          "comments": { "nodes": [
            { "databaseId": 1583234287, "body": "Lol not suspicious of coupling",
              "path": "pkg/cmdutil/auth_check_test.go", "line": 113, "originalLine": 113,
              "createdAt": "2024-04-29T14:59:46Z", "author": { "login": "williammartin" } }
          ] } }
      ] },
      "comments": { "nodes": [
        { "databaseId": 2080158371, "body": "@steiza @phillmv : is the manu",
          "createdAt": "2024-04-26T21:44:55Z", "author": { "login": "andyfeller" } }
      ] }
    }"#;

    fn threads() -> Vec<MrThread> {
        threads_from_graphql(&serde_json::from_str(THREADS).unwrap())
    }

    #[test]
    fn every_thread_and_the_conversation_become_rows() {
        assert_eq!(threads().len(), 3);
    }

    #[test]
    fn a_thread_keeps_its_node_id() {
        let out = threads();
        assert_eq!(out[0].id, "PRRT_kwDODKw3uc48Rk4m");
        assert_eq!(out[1].id, "PRRT_kwDODKw3uc48RxjH");
    }

    #[test]
    fn a_note_carries_its_author_and_new_side_position() {
        let out = threads();
        let note = &out[0].notes[0];
        assert_eq!(note.author, "williammartin");
        let pos = note.position.as_ref().unwrap();
        assert_eq!(
            pos.new_path.as_deref(),
            Some("pkg/cmd/attestation/verify/verify.go")
        );
        assert_eq!(pos.new_line, Some(130));
        assert!(note.resolved);
        assert!(note.resolvable);
    }

    #[test]
    fn github_notes_carry_no_old_side_and_no_range() {
        let out = threads();
        let pos = out[0].notes[0].position.as_ref().unwrap();
        assert_eq!(pos.old_path, None);
        assert_eq!(pos.old_line, None);
        assert_eq!(pos.end_new_line, None);
    }

    #[test]
    fn an_open_thread_reads_as_unresolved() {
        assert!(!threads()[1].notes[0].resolved);
    }

    #[test]
    fn a_conversation_comment_has_no_position_and_no_resolve() {
        let out = threads();
        let note = &out[2].notes[0];
        assert!(note.position.is_none());
        assert!(!note.resolvable);
        assert_eq!(note.author, "andyfeller");
    }

    #[test]
    fn an_old_side_thread_keeps_the_note_but_drops_the_position() {
        let raw = serde_json::json!({
          "reviewThreads": { "nodes": [
            { "id": "T1", "isResolved": false, "path": "a.rs", "line": 5, "diffSide": "LEFT",
              "comments": { "nodes": [
                { "databaseId": 1, "body": "b", "line": 5, "author": { "login": "x" } }
              ] } }
          ] },
          "comments": { "nodes": [] }
        });
        let out = threads_from_graphql(&raw);
        assert_eq!(out[0].notes[0].body, "b");
        assert!(out[0].notes[0].position.is_none());
    }

    #[test]
    fn an_empty_pull_request_yields_no_threads() {
        let raw =
            serde_json::json!({ "reviewThreads": { "nodes": [] }, "comments": { "nodes": [] } });
        assert!(threads_from_graphql(&raw).is_empty());
        // A payload with no keys must not panic.
        assert!(threads_from_graphql(&serde_json::json!({})).is_empty());
    }

    #[test]
    fn statuses_are_the_ones_the_ui_groups_on() {
        for r in [
            serde_json::json!([{ "conclusion": "FAILURE" }]),
            serde_json::json!([{ "status": "IN_PROGRESS" }]),
            serde_json::json!([{ "conclusion": "SUCCESS" }]),
        ] {
            let (status, _) = rollup_status(&r).unwrap();
            assert!(
                ["failed", "running", "success"].contains(&status.as_str()),
                "{status}"
            );
        }
    }
}
