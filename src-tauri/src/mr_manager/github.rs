//! GitHub through the `gh` CLI, feature-for-feature with the GitLab client.
//!
//! `gh` rather than the REST API directly, for the same reason `glab` is used for
//! GitLab: auth is the CLI's problem, not ours. The previous implementation held a
//! token in the OS keyring with no UI to put it there, and three features were
//! simply missing — CI status, thread resolution, and review threads in the shape
//! the UI reads (it returned flat review comments, so threads rendered as nothing).
//!
//! Review threads come from GraphQL because REST cannot express them: resolution
//! state and the thread node id needed to resolve one exist only there. Everything
//! else uses the `gh pr` porcelain.

use crate::core::db::models::Repo;
use super::platform::{detect_default_branch, CliMissing, PlatformClient};

/// Run `gh` in a repo checkout. Like `glab_run`, the cwd is what tells `gh` which
/// repository (and host) it is talking about.
pub(super) async fn gh_run(cwd: String, args: Vec<String>) -> anyhow::Result<String> {
    let printable = args.join(" ");
    let out = crate::core::timing::timed("subprocess", format!("gh {printable}"), async {
        tokio::task::spawn_blocking(move || {
            std::process::Command::new("gh")
                .args(&args)
                .current_dir(&cwd)
                // gh paginates interactively and colours output when it thinks it has a
                // terminal; both corrupt JSON parsing.
                .env("GH_PAGER", "")
                .env("NO_COLOR", "1")
                .output()
        })
        .await
    })
    .await?;

    let out = match out {
        Ok(o) => o,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(anyhow::Error::new(CliMissing("gh")));
        }
        Err(e) => return Err(anyhow::anyhow!("failed to run gh {printable}: {e}")),
    };

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        // The one failure worth translating: everything fails this way until the
        // user logs in, and gh's own wording buries it.
        if stderr.contains("not logged") || stderr.contains("authentication") {
            return Err(anyhow::anyhow!("`gh` is not authenticated — run `gh auth login`"));
        }
        return Err(anyhow::anyhow!("gh {printable}: {stderr}"));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

/// `gh pr view <n> --json …` for the fields a caller names.
async fn pr_json(repo: &Repo, remote_id: &str, fields: &str) -> anyhow::Result<serde_json::Value> {
    let out = gh_run(
        repo.local_path.clone(),
        vec![
            "pr".into(), "view".into(), remote_id.to_string(),
            "--json".into(), fields.to_string(),
        ],
    )
    .await?;
    Ok(serde_json::from_str(&out)?)
}

/// A GraphQL call against the repo's own host. `-F` sends typed variables, so
/// numbers stay numbers (`-f` would make the PR number a string and the query
/// would fail its type check).
async fn graphql(
    cwd: &str,
    query: &str,
    vars: &[(&str, String)],
) -> anyhow::Result<serde_json::Value> {
    let mut args = vec!["api".to_string(), "graphql".into(), "-f".into(), format!("query={query}")];
    for (k, v) in vars {
        args.push("-F".into());
        args.push(format!("{k}={v}"));
    }
    let out = gh_run(cwd.to_string(), args).await?;
    let v: serde_json::Value = serde_json::from_str(&out)?;
    if let Some(errors) = v["errors"].as_array() {
        if !errors.is_empty() {
            let msg = errors
                .iter()
                .filter_map(|e| e["message"].as_str())
                .collect::<Vec<_>>()
                .join("; ");
            return Err(anyhow::anyhow!("GitHub GraphQL: {msg}"));
        }
    }
    Ok(v)
}

/// `owner` and `repo` as GitHub means them: the group path can be nested in the
/// local MAIN layout, but on GitHub only the last segment before the project is the
/// owner (`github.com/<owner>/<repo>`).
fn owner_of(repo: &Repo) -> &str {
    repo.group_path.rsplit('/').next().unwrap_or(&repo.group_path)
}

/// The current login, for "did I approve?". Stable for a run, like the glab one.
static GH_LOGIN: std::sync::OnceLock<String> = std::sync::OnceLock::new();

async fn gh_login(cwd: &str) -> Option<String> {
    if let Some(l) = GH_LOGIN.get() {
        return Some(l.clone());
    }
    let out = gh_run(cwd.to_string(), vec!["api".into(), "user".into(), "--jq".into(), ".login".into()])
        .await
        .ok()?;
    let login = out.trim().to_string();
    if login.is_empty() {
        return None;
    }
    let _ = GH_LOGIN.set(login.clone());
    Some(login)
}

/// GitHub reports a check per job; the UI wants one word. Worst state wins, which
/// is what a reviewer needs to know.
fn rollup_status(rollup: &serde_json::Value) -> Option<(String, String)> {
    let nodes = rollup.as_array()?;
    if nodes.is_empty() {
        return None;
    }
    let mut running = false;
    let mut failed = false;
    let mut url = String::new();
    for n in nodes {
        // A check run reports `status` + `conclusion`; a legacy commit status
        // reports `state`. Both appear in the same rollup.
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
            (_, "FAILURE" | "TIMED_OUT" | "CANCELLED" | "ACTION_REQUIRED" | "STARTUP_FAILURE", _)
            | (_, _, "FAILURE" | "ERROR") => failed = true,
            ("QUEUED" | "IN_PROGRESS" | "PENDING" | "WAITING" | "REQUESTED", _, _)
            | (_, _, "PENDING") => running = true,
            _ => {}
        }
    }
    // The vocabulary is GitLab's, because the UI groups on it (see ciGroup).
    let status = if failed { "failed" } else if running { "running" } else { "success" };
    Some((status.to_string(), url))
}

/// GraphQL review threads → the `[{ id, notes: [...] }]` shape the UI reads.
///
/// Kept pure and separate from the fetch so a captured payload can pin it: the UI
/// silently renders nothing when this drifts (which is how the REST version shipped
/// broken).
fn threads_from_graphql(pr: &serde_json::Value) -> serde_json::Value {
    let mut threads = vec![];
    for t in pr["reviewThreads"]["nodes"].as_array().cloned().unwrap_or_default() {
        let resolved = t["isResolved"].as_bool().unwrap_or(false);
        // A LEFT-side thread is on the old file; the UI anchors on new-side lines
        // only, so the note is kept but without a position.
        let on_new_side = t["diffSide"].as_str().unwrap_or("RIGHT") == "RIGHT";
        let path = t["path"].as_str().unwrap_or("").to_string();
        let notes: Vec<serde_json::Value> = t["comments"]["nodes"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .map(|c| {
                let line = c["line"].as_i64().or(c["originalLine"].as_i64());
                let mut note = serde_json::json!({
                    "id": c["databaseId"],
                    "body": c["body"].as_str().unwrap_or(""),
                    "author": { "username": c["author"]["login"].as_str().unwrap_or("") },
                    "created_at": c["createdAt"].as_str().unwrap_or(""),
                    "resolved": resolved,
                    "resolvable": true,
                });
                if on_new_side {
                    if let Some(line) = line {
                        note["position"] = serde_json::json!({
                            "new_path": path,
                            "new_line": line,
                        });
                    }
                }
                note
            })
            .collect();
        if notes.is_empty() {
            continue;
        }
        threads.push(serde_json::json!({ "id": t["id"], "notes": notes }));
    }

    // Conversation comments: no position, not resolvable — the same treatment a
    // GitLab general note gets.
    for c in pr["comments"]["nodes"].as_array().cloned().unwrap_or_default() {
        threads.push(serde_json::json!({
            "id": c["databaseId"].to_string(),
            "notes": [{
                "id": c["databaseId"],
                "body": c["body"].as_str().unwrap_or(""),
                "author": { "username": c["author"]["login"].as_str().unwrap_or("") },
                "created_at": c["createdAt"].as_str().unwrap_or(""),
                "resolved": true,
                "resolvable": false,
            }],
        }));
    }
    serde_json::Value::Array(threads)
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
        title: &str,
        description: &str,
    ) -> anyhow::Result<(String, String)> {
        // The real default branch, not a hardcoded "main" as before.
        let base = detect_default_branch(&repo.local_path).await;
        let out = gh_run(
            repo.local_path.clone(),
            vec![
                "pr".into(), "create".into(),
                "--head".into(), branch.to_string(),
                "--base".into(), base,
                "--title".into(), title.to_string(),
                "--body".into(), description.to_string(),
                "--assignee".into(), "@me".into(),
            ],
        )
        .await?;

        let url = out
            .lines()
            .find_map(|l| {
                let pos = l.find("https://")?;
                Some(l[pos..].split_whitespace().next().unwrap_or("").to_string())
            })
            .ok_or_else(|| anyhow::anyhow!("no URL in gh pr create output:\n{out}"))?;
        let number = url
            .rsplit('/')
            .next()
            .ok_or_else(|| anyhow::anyhow!("could not parse a PR number from URL: {url}"))?
            .to_string();
        Ok((number, url))
    }

    async fn update_mr(
        &self,
        repo: &Repo,
        remote_id: &str,
        title: Option<&str>,
        description: Option<&str>,
    ) -> anyhow::Result<()> {
        let mut args = vec!["pr".to_string(), "edit".into(), remote_id.to_string()];
        if let Some(t) = title {
            args.push("--title".into());
            args.push(t.to_string());
        }
        if let Some(d) = description {
            args.push("--body".into());
            args.push(d.to_string());
        }
        gh_run(repo.local_path.clone(), args).await?;
        Ok(())
    }

    async fn close_mr(&self, repo: &Repo, remote_id: &str) -> anyhow::Result<()> {
        gh_run(
            repo.local_path.clone(),
            vec!["pr".into(), "close".into(), remote_id.to_string()],
        )
        .await?;
        Ok(())
    }

    async fn get_mr_details(&self, repo: &Repo, remote_id: &str) -> anyhow::Result<serde_json::Value> {
        let v = pr_json(
            repo,
            remote_id,
            "title,body,author,headRefName,baseRefName,state,isDraft,createdAt,url",
        )
        .await?;
        Ok(serde_json::json!({
            "title": v["title"].as_str().unwrap_or(""),
            "description": v["body"].as_str().unwrap_or(""),
            "author": v["author"]["login"].as_str().or(v["author"]["name"].as_str()).unwrap_or(""),
            "source_branch": v["headRefName"].as_str().unwrap_or(""),
            "target_branch": v["baseRefName"].as_str().unwrap_or(""),
            // gh reports OPEN / CLOSED / MERGED; the UI expects lower case.
            "state": v["state"].as_str().unwrap_or("OPEN").to_lowercase(),
            "draft": v["isDraft"].as_bool().unwrap_or(false),
            "created_at": v["createdAt"].as_str().unwrap_or(""),
            "web_url": v["url"].as_str().unwrap_or(""),
        }))
    }

    async fn get_mr_ci(&self, repo: &Repo, remote_id: &str) -> anyhow::Result<serde_json::Value> {
        let v = pr_json(repo, remote_id, "statusCheckRollup,url").await?;
        let Some((status, url)) = rollup_status(&v["statusCheckRollup"]) else {
            return Ok(serde_json::Value::Null);
        };
        Ok(serde_json::json!({
            "status": status,
            "url": if url.is_empty() { v["url"].as_str().unwrap_or("") } else { &url },
        }))
    }

    /// Review threads, in the shape the UI reads (`[{ id, notes: [...] }]`).
    ///
    /// GraphQL is the only source for `isResolved` and for the thread id that
    /// `resolve_mr_thread` needs. General PR comments are appended as
    /// position-less single-note threads, matching how GitLab returns notes.
    async fn get_mr_threads(
        &self,
        repo: &Repo,
        remote_id: &str,
    ) -> anyhow::Result<serde_json::Value> {
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
        let v = graphql(
            &repo.local_path,
            QUERY,
            &[
                ("owner", owner_of(repo).to_string()),
                ("name", repo.project.clone()),
                ("number", remote_id.to_string()),
            ],
        )
        .await?;
        Ok(threads_from_graphql(&v["data"]["repository"]["pullRequest"]))
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
        graphql(
            &repo.local_path,
            MUTATION,
            &[("threadId", thread_id.to_string()), ("body", body.to_string())],
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
        let v = graphql(&repo.local_path, MUTATION, &[("threadId", thread_id.to_string())]).await?;
        if v["data"]["resolveReviewThread"]["thread"]["isResolved"].as_bool() == Some(true) {
            return Ok(());
        }
        Err(anyhow::anyhow!("GitHub did not report the thread as resolved"))
    }

    async fn approve_mr(&self, repo: &Repo, remote_id: &str) -> anyhow::Result<()> {
        gh_run(
            repo.local_path.clone(),
            vec!["pr".into(), "review".into(), remote_id.to_string(), "--approve".into()],
        )
        .await?;
        Ok(())
    }

    async fn get_mr_approval(&self, repo: &Repo, remote_id: &str) -> anyhow::Result<serde_json::Value> {
        let v = pr_json(repo, remote_id, "reviews,reviewDecision").await?;
        // Latest review per author wins: an APPROVED then CHANGES_REQUESTED must
        // not still count as an approval.
        let mut latest: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        for r in v["reviews"].as_array().cloned().unwrap_or_default() {
            let Some(login) = r["author"]["login"].as_str() else { continue };
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
        let me = gh_login(&repo.local_path).await;
        Ok(serde_json::json!({
            "approved": v["reviewDecision"].as_str() == Some("APPROVED") || !by.is_empty(),
            "approved_by_me": me.map(|m| by.contains(&m)).unwrap_or(false),
            "approved_by": by,
        }))
    }

    async fn post_mr_comment(
        &self,
        repo: &Repo,
        remote_id: &str,
        body: &str,
        position: Option<(&str, i64)>,
    ) -> anyhow::Result<()> {
        let Some((path, line)) = position else {
            gh_run(
                repo.local_path.clone(),
                vec![
                    "pr".into(), "comment".into(), remote_id.to_string(),
                    "--body".into(), body.to_string(),
                ],
            )
            .await?;
            return Ok(());
        };

        // A positioned review comment needs the PR head sha. Like GitLab, the
        // position references the REMOTE head, so local commits can drift it.
        let head = pr_json(repo, remote_id, "headRefOid").await?;
        let head_sha = head["headRefOid"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("PR #{remote_id} has no head sha"))?;

        let endpoint = format!(
            "repos/{}/{}/pulls/{remote_id}/comments",
            owner_of(repo),
            repo.project
        );
        gh_run(
            repo.local_path.clone(),
            vec![
                "api".into(), endpoint,
                "--method".into(), "POST".into(),
                "-f".into(), format!("body={body}"),
                "-f".into(), format!("commit_id={head_sha}"),
                "-f".into(), format!("path={path}"),
                "-F".into(), format!("line={line}"),
                "-f".into(), "side=RIGHT".into(),
            ],
        )
        .await?;
        Ok(())
    }
}

/// Open PRs where the current user is a requested reviewer, for the review queue.
///
/// One GraphQL search rather than `gh search prs`, whose JSON has no branch names —
/// and the review session needs both refs to check the PR out. `reviewDecision`
/// comes back in the same call, so approval costs no extra round-trip (GitLab needs
/// one per MR).
pub(super) async fn review_requested_prs(cwd: &str) -> anyhow::Result<Vec<serde_json::Value>> {
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
    let v = graphql(cwd, QUERY, &[]).await?;
    Ok(v["data"]["search"]["nodes"].as_array().cloned().unwrap_or_default())
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

    // The MAIN layout can nest a group path, but GitHub owns exactly one segment.
    #[test]
    fn owner_is_the_last_group_segment() {
        assert_eq!(owner_of(&repo("cli", "cli")), "cli");
        assert_eq!(owner_of(&repo("acme/team", "svc")), "team");
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

    // Worst state wins: a reviewer needs to know something is broken even when
    // most jobs passed.
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

    /// Legacy commit statuses report `state`, not `status`/`conclusion`.
    #[test]
    fn reads_legacy_commit_statuses_too() {
        let failed = serde_json::json!([{ "state": "FAILURE", "targetUrl": "https://ci/2" }]);
        assert_eq!(rollup_status(&failed).unwrap(), ("failed".into(), "https://ci/2".into()));
        let pending = serde_json::json!([{ "state": "PENDING" }]);
        assert_eq!(rollup_status(&pending).unwrap().0, "running");
        let ok = serde_json::json!([{ "state": "SUCCESS" }]);
        assert_eq!(rollup_status(&ok).unwrap().0, "success");
    }

    /// Captured from `gh api graphql` against cli/cli#9000 — a resolved thread, an
    /// open one, and a conversation comment.
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

    fn threads() -> serde_json::Value {
        threads_from_graphql(&serde_json::from_str(THREADS).unwrap())
    }

    #[test]
    fn every_thread_and_the_conversation_become_rows() {
        let out = threads();
        assert_eq!(out.as_array().unwrap().len(), 3);
    }

    /// The thread id must be the GraphQL NODE id: `resolve_mr_thread` and
    /// `reply_to_thread` both address the thread by it.
    #[test]
    fn a_thread_keeps_its_node_id() {
        let out = threads();
        assert_eq!(out[0]["id"], "PRRT_kwDODKw3uc48Rk4m");
        assert_eq!(out[1]["id"], "PRRT_kwDODKw3uc48RxjH");
    }

    // The shape the UI reads: notes[0].author.username + position.new_path/new_line.
    #[test]
    fn a_note_carries_its_author_and_new_side_position() {
        let out = threads();
        let note = &out[0]["notes"][0];
        assert_eq!(note["author"]["username"], "williammartin");
        assert_eq!(note["position"]["new_path"], "pkg/cmd/attestation/verify/verify.go");
        assert_eq!(note["position"]["new_line"], 130);
        assert_eq!(note["resolved"], true);
        assert_eq!(note["resolvable"], true);
    }

    #[test]
    fn an_open_thread_reads_as_unresolved() {
        assert_eq!(threads()[1]["notes"][0]["resolved"], false);
    }

    /// A conversation comment is a position-less, non-resolvable note — otherwise
    /// the UI offers a Resolve button that GitHub would reject.
    #[test]
    fn a_conversation_comment_has_no_position_and_no_resolve() {
        let out = threads();
        let note = &out[2]["notes"][0];
        assert!(note["position"].is_null());
        assert_eq!(note["resolvable"], false);
        assert_eq!(note["author"]["username"], "andyfeller");
    }

    /// LEFT-side threads are on the old file, which the UI cannot anchor.
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
        assert_eq!(out[0]["notes"][0]["body"], "b");
        assert!(out[0]["notes"][0]["position"].is_null());
    }

    #[test]
    fn an_empty_pull_request_yields_no_threads() {
        let raw = serde_json::json!({ "reviewThreads": { "nodes": [] }, "comments": { "nodes": [] } });
        assert_eq!(threads_from_graphql(&raw).as_array().unwrap().len(), 0);
        // A payload missing the keys entirely (an errored query) must not panic.
        assert_eq!(threads_from_graphql(&serde_json::json!({})).as_array().unwrap().len(), 0);
    }

    // The vocabulary is GitLab's on purpose — the UI's ciGroup() maps these.
    #[test]
    fn statuses_are_the_ones_the_ui_groups_on() {
        for r in [
            serde_json::json!([{ "conclusion": "FAILURE" }]),
            serde_json::json!([{ "status": "IN_PROGRESS" }]),
            serde_json::json!([{ "conclusion": "SUCCESS" }]),
        ] {
            let (status, _) = rollup_status(&r).unwrap();
            assert!(["failed", "running", "success"].contains(&status.as_str()), "{status}");
        }
    }
}
