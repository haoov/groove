use crate::db::schema::{Mr, Repo, Worktree};
use sqlx::SqlitePool;
use super::platform::{detect_default_branch, PlatformClient};

pub(super) async fn glab_run(cwd: String, args: Vec<String>) -> anyhow::Result<String> {
    let printable = args.join(" ");
    let out = tokio::task::spawn_blocking(move || {
        std::process::Command::new("glab")
            .args(&args)
            .current_dir(&cwd)
            .output()
    })
    .await?;

    let out = match out {
        Ok(o) => o,
        // Typed, so the review queue can tell "no GitLab here" from "GitLab broke".
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(anyhow::Error::new(super::platform::CliMissing("glab")));
        }
        Err(e) => return Err(anyhow::anyhow!("failed to run glab {printable}: {e}")),
    };

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        if stderr.contains("not logged") || stderr.contains("authentication") {
            return Err(anyhow::anyhow!("`glab` is not authenticated — run `glab auth login`"));
        }
        return Err(anyhow::anyhow!("glab: {stderr}"));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

pub(super) fn glab_state(raw: &str) -> String {
    match raw {
        "opened" => "open".to_string(),
        "merged" => "merged".to_string(),
        "closed" => "closed".to_string(),
        other => other.to_string(),
    }
}

/// One `glab mr view <iid> --output json` fetch — shared by CI + details.
async fn mr_view_json(repo: &Repo, remote_id: &str) -> anyhow::Result<serde_json::Value> {
    let out = glab_run(
        repo.local_path.clone(),
        vec![
            "mr".into(),
            "view".into(),
            remote_id.to_string(),
            "--output".into(),
            "json".into(),
        ],
    )
    .await?;
    Ok(serde_json::from_str(&out)?)
}

/// Approval state for an MR addressed by project path — used by the review queue,
/// which lists MRs across projects and so has no `Repo` row to hand a client.
/// `cwd` only has to be *some* repo glab can resolve a host from.
pub(super) async fn mr_approved(cwd: &str, project_full: &str, iid: u64) -> bool {
    let endpoint = format!(
        "projects/{}/merge_requests/{iid}/approvals",
        project_full.replace('/', "%2F")
    );
    glab_run(cwd.to_string(), vec!["api".into(), endpoint])
        .await
        .ok()
        .and_then(|out| serde_json::from_str::<serde_json::Value>(&out).ok())
        .and_then(|v| {
            v["approved"].as_bool().or_else(|| {
                v["approved_by"].as_array().map(|a| !a.is_empty())
            })
        })
        .unwrap_or(false)
}

pub(super) struct GlabClient;

#[async_trait::async_trait]
impl PlatformClient for GlabClient {
    fn platform_name(&self) -> &'static str {
        "gitlab"
    }

    async fn create_mr(
        &self,
        repo: &Repo,
        branch: &str,
        title: &str,
        description: &str,
    ) -> anyhow::Result<(String, String)> {
        let default_branch = detect_default_branch(&repo.local_path).await;
        let out = glab_run(
            repo.local_path.clone(),
            vec![
                "mr".into(),
                "create".into(),
                "--source-branch".into(),
                branch.to_string(),
                "--target-branch".into(),
                default_branch,
                "--title".into(),
                title.to_string(),
                "--description".into(),
                description.to_string(),
                "--assignee".into(),
                "@me".into(),
                "--squash-before-merge".into(),
                "--remove-source-branch".into(),
                "--yes".into(),
            ],
        )
        .await?;

        let url = out
            .lines()
            .find_map(|l| {
                let pos = l.find("https://")?;
                Some(l[pos..].split_whitespace().next().unwrap_or("").to_string())
            })
            .ok_or_else(|| anyhow::anyhow!("no URL in glab mr create output:\n{out}"))?;

        let iid = url
            .split('/')
            .next_back()
            .ok_or_else(|| anyhow::anyhow!("could not parse IID from URL: {url}"))?
            .to_string();

        Ok((iid, url))
    }

    async fn update_mr(
        &self,
        repo: &Repo,
        remote_id: &str,
        title: Option<&str>,
        description: Option<&str>,
    ) -> anyhow::Result<()> {
        let mut args = vec!["mr".to_string(), "update".to_string(), remote_id.to_string()];
        if let Some(t) = title {
            args.push("--title".into());
            args.push(t.to_string());
        }
        if let Some(d) = description {
            args.push("--description".into());
            args.push(d.to_string());
        }
        glab_run(repo.local_path.clone(), args).await?;
        Ok(())
    }

    async fn close_mr(&self, repo: &Repo, remote_id: &str) -> anyhow::Result<()> {
        glab_run(
            repo.local_path.clone(),
            vec!["mr".into(), "close".into(), remote_id.to_string()],
        )
        .await?;
        Ok(())
    }

    async fn get_mr_threads(
        &self,
        repo: &Repo,
        remote_id: &str,
    ) -> anyhow::Result<serde_json::Value> {
        let out = glab_run(
            repo.local_path.clone(),
            vec![
                "mr".into(),
                "note".into(),
                "list".into(),
                remote_id.to_string(),
                "--output".into(),
                "json".into(),
            ],
        )
        .await?;
        Ok(serde_json::from_str(&out).unwrap_or(serde_json::json!([])))
    }

    async fn get_mr_ci(&self, repo: &Repo, remote_id: &str) -> anyhow::Result<serde_json::Value> {
        let v = mr_view_json(repo, remote_id).await?;
        let p = if v["head_pipeline"].is_object() {
            &v["head_pipeline"]
        } else {
            &v["pipeline"]
        };
        if !p.is_object() {
            return Ok(serde_json::Value::Null);
        }
        Ok(serde_json::json!({
            "status": p["status"].as_str().unwrap_or("unknown"),
            "url": p["web_url"].as_str().unwrap_or(""),
        }))
    }

    async fn get_mr_details(&self, repo: &Repo, remote_id: &str) -> anyhow::Result<serde_json::Value> {
        let v = mr_view_json(repo, remote_id).await?;
        Ok(serde_json::json!({
            "title": v["title"].as_str().unwrap_or(""),
            "description": v["description"].as_str().unwrap_or(""),
            "author": v["author"]["username"].as_str().or(v["author"]["name"].as_str()).unwrap_or(""),
            "source_branch": v["source_branch"].as_str().unwrap_or(""),
            "target_branch": v["target_branch"].as_str().unwrap_or(""),
            "state": glab_state(v["state"].as_str().unwrap_or("opened")),
            "draft": v["draft"].as_bool().or(v["work_in_progress"].as_bool()).unwrap_or(false),
            "created_at": v["created_at"].as_str().unwrap_or(""),
            "web_url": v["web_url"].as_str().unwrap_or(""),
        }))
    }

    async fn reply_to_thread(
        &self,
        repo: &Repo,
        remote_id: &str,
        _thread_id: &str,
        body: &str,
    ) -> anyhow::Result<()> {
        glab_run(
            repo.local_path.clone(),
            vec![
                "mr".into(),
                "note".into(),
                "create".into(),
                remote_id.to_string(),
                "--message".into(),
                body.to_string(),
            ],
        )
        .await?;
        Ok(())
    }

    async fn resolve_mr_thread(
        &self,
        repo: &Repo,
        remote_id: &str,
        thread_id: &str,
    ) -> anyhow::Result<()> {
        let project_path = format!("{}/{}", repo.group_path, repo.project)
            .replace('/', "%2F");
        let endpoint = format!(
            "projects/{project_path}/merge_requests/{remote_id}/discussions/{thread_id}"
        );
        glab_run(
            repo.local_path.clone(),
            vec![
                "api".into(),
                endpoint,
                "--method".into(),
                "PUT".into(),
                "-F".into(),
                "resolved=true".into(),
            ],
        )
        .await?;
        Ok(())
    }

    async fn approve_mr(&self, repo: &Repo, remote_id: &str) -> anyhow::Result<()> {
        glab_run(
            repo.local_path.clone(),
            vec!["mr".into(), "approve".into(), remote_id.to_string()],
        )
        .await?;
        Ok(())
    }

    async fn get_mr_approval(&self, repo: &Repo, remote_id: &str) -> anyhow::Result<serde_json::Value> {
        let project_path = format!("{}/{}", repo.group_path, repo.project).replace('/', "%2F");
        let endpoint = format!("projects/{project_path}/merge_requests/{remote_id}/approvals");
        let out = glab_run(repo.local_path.clone(), vec!["api".into(), endpoint]).await?;
        let v: serde_json::Value = serde_json::from_str(&out)?;
        let by: Vec<String> = v["approved_by"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|a| a["user"]["username"].as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        Ok(serde_json::json!({
            "approved": v["approved"].as_bool().unwrap_or(!by.is_empty()),
            // GitLab reports this per-token, so it answers "did I approve?".
            "approved_by_me": v["user_has_approved"].as_bool().unwrap_or(false),
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
        let Some((new_path, new_line)) = position else {
            // General note — same surface the reply flow uses.
            glab_run(
                repo.local_path.clone(),
                vec![
                    "mr".into(),
                    "note".into(),
                    "create".into(),
                    remote_id.to_string(),
                    "--message".into(),
                    body.to_string(),
                ],
            )
            .await?;
            return Ok(());
        };

        // Positioned discussion: GitLab needs the MR's diff_refs shas. Caveat:
        // positions reference the REMOTE MR head — local commits in the review
        // worktree can drift line numbers, so post before editing.
        let v = mr_view_json(repo, remote_id).await?;
        let refs = &v["diff_refs"];
        let (Some(base_sha), Some(start_sha), Some(head_sha)) = (
            refs["base_sha"].as_str(),
            refs["start_sha"].as_str(),
            refs["head_sha"].as_str(),
        ) else {
            return Err(anyhow::anyhow!("MR !{remote_id} has no diff_refs — cannot position the comment"));
        };

        let project_path = format!("{}/{}", repo.group_path, repo.project).replace('/', "%2F");
        let endpoint = format!("projects/{project_path}/merge_requests/{remote_id}/discussions");

        // Read the MR's diff to learn what kind of line this is before posting.
        // Guessing can't work: GitLab needs old+new for an unchanged line, only
        // one of them for an added or deleted line, and refuses the request with
        // `line_code: must be a valid line code` when the pair doesn't match.
        let anchor = locate_line(repo, &project_path, remote_id, new_path, new_line).await?;
        let created =
            post_positioned(repo, &endpoint, body, &anchor, base_sha, start_sha, head_sha).await?;

        // Verify the anchor actually landed. A note without a `position` IS the
        // context-less comment this method exists to avoid, so roll it back and
        // report rather than leaving it on the MR.
        let discussion: serde_json::Value = serde_json::from_str(&created).unwrap_or_default();
        if discussion["notes"][0]["position"].is_object() {
            return Ok(());
        }

        let discussion_id = discussion["id"].as_str().unwrap_or("").to_string();
        let note_id = discussion["notes"][0]["id"].as_i64();
        if let (false, Some(note_id)) = (discussion_id.is_empty(), note_id) {
            let _ = glab_run(
                repo.local_path.clone(),
                vec![
                    "api".into(),
                    format!("{endpoint}/{discussion_id}/notes/{note_id}"),
                    "--method".into(),
                    "DELETE".into(),
                ],
            )
            .await;
        }
        Err(anyhow::anyhow!(
            "GitLab did not anchor the comment to {new_path}:{new_line} — that line may not be \
             part of this MR's diff, or the MR head moved. Nothing was posted."
        ))
    }
}

/// Where one line of a file sits in an MR's diff.
///
/// GitLab derives a comment's `line_code` from the PAIR of line numbers, so which
/// fields a position must carry depends on what kind of line it is:
///   added     → `new_line` only (there is no old line)
///   deleted   → `old_line` only (there is no new line)
///   unchanged → BOTH, because a context line exists on both sides
/// Sending one number for an unchanged line is what produced
/// `Note {:line_code=>["can't be blank", "must be a valid line code"]}` — and most
/// review comments land on context lines, so this is the common case, not an edge.
struct DiffAnchor {
    old_line: Option<i64>,
    new_line: Option<i64>,
    old_path: String,
    new_path: String,
}

/// Locate `line` in the MR's diff for `path`.
///
/// `line` is a new-side number by our own convention, but an annotation on a
/// DELETED line can only carry an old-side one, so the new side is tried first and
/// the old side second. Returns the paths from the diff too, which is what makes
/// comments work on files renamed inside the MR.
async fn locate_line(
    repo: &Repo,
    project_path: &str,
    remote_id: &str,
    path: &str,
    line: i64,
) -> anyhow::Result<DiffAnchor> {
    let raw = glab_run(
        repo.local_path.clone(),
        vec![
            "api".into(),
            format!("projects/{project_path}/merge_requests/{remote_id}/diffs?per_page=100"),
        ],
    )
    .await?;
    let files: Vec<serde_json::Value> = serde_json::from_str(&raw)
        .map_err(|e| anyhow::anyhow!("could not read MR !{remote_id}'s diff: {e}"))?;

    let file = files
        .iter()
        .find(|f| f["new_path"].as_str() == Some(path) || f["old_path"].as_str() == Some(path))
        .ok_or_else(|| {
            anyhow::anyhow!("{path} is not part of MR !{remote_id} — nothing was posted.")
        })?;

    let old_path = file["old_path"].as_str().unwrap_or(path).to_string();
    let new_path = file["new_path"].as_str().unwrap_or(path).to_string();
    let diff = file["diff"].as_str().unwrap_or_default();

    match resolve_in_diff(diff, line) {
        Ok((old_line, new_line)) => Ok(DiffAnchor { old_line, new_line, old_path, new_path }),
        Err(commentable) => Err(anyhow::anyhow!(
            "line {line} of {path} is not in MR !{remote_id}'s diff, so GitLab has nothing to \
             anchor to — comment on one of its changed regions instead ({}). Nothing was posted.",
            describe_ranges(&commentable)
        )),
    }
}

/// Find `line` in a unified diff and return the `(old_line, new_line)` pair
/// GitLab needs. On failure, hands back every new-side line that IS commentable.
///
/// `line` is treated as a new-side number first (our convention) and as an
/// old-side one second, which is the only way to anchor a comment on a line the
/// MR deleted.
#[allow(clippy::type_complexity)]
fn resolve_in_diff(diff: &str, line: i64) -> Result<(Option<i64>, Option<i64>), Vec<i64>> {
    let mut old_no = 0i64;
    let mut new_no = 0i64;
    let mut by_new: Vec<(i64, Option<i64>)> = vec![]; // (new_line, old_line if unchanged)
    let mut by_old: Vec<i64> = vec![]; // deleted lines

    for raw_line in diff.lines() {
        if let Some((old, new)) = parse_hunk_header(raw_line) {
            old_no = old;
            new_no = new;
            continue;
        }
        match raw_line.chars().next() {
            Some('+') => {
                by_new.push((new_no, None));
                new_no += 1;
            }
            Some('-') => {
                by_old.push(old_no);
                old_no += 1;
            }
            // `\ No newline at end of file` is metadata, not a line.
            Some('\\') => {}
            // A context line — present on both sides, so it carries both numbers.
            Some(_) | None => {
                by_new.push((new_no, Some(old_no)));
                old_no += 1;
                new_no += 1;
            }
        }
    }

    if let Some((_, old)) = by_new.iter().find(|(n, _)| *n == line) {
        return Ok((*old, Some(line)));
    }
    if by_old.contains(&line) {
        return Ok((Some(line), None));
    }
    Err(by_new.iter().map(|(n, _)| *n).collect())
}

/// `@@ -old,count +new,count @@` → the two starting line numbers.
fn parse_hunk_header(line: &str) -> Option<(i64, i64)> {
    let rest = line.strip_prefix("@@ -")?;
    let (old, rest) = rest.split_once(" +")?;
    let new = rest.split_once(" @@").map(|(n, _)| n).unwrap_or(rest);
    let first = |s: &str| s.split(',').next().unwrap_or(s).parse::<i64>().ok();
    Some((first(old)?, first(new)?))
}

/// Collapse line numbers into "12-40, 88-96" for an actionable error message.
fn describe_ranges(lines: &[i64]) -> String {
    let mut out: Vec<String> = vec![];
    let mut start: Option<i64> = None;
    let mut prev = 0i64;
    for &n in lines {
        match start {
            None => start = Some(n),
            Some(s) if n != prev + 1 => {
                out.push(if s == prev { s.to_string() } else { format!("{s}-{prev}") });
                start = Some(n);
            }
            _ => {}
        }
        prev = n;
    }
    if let Some(s) = start {
        out.push(if s == prev { s.to_string() } else { format!("{s}-{prev}") });
    }
    if out.is_empty() { "none".to_string() } else { out.join(", ") }
}

/// POST one positioned discussion. Returns the raw response body.
///
/// `--form`, NOT `-f`: `-f` builds a JSON body where `position[new_path]` stays a
/// FLAT key that GitLab ignores — it then created a context-less note instead of
/// failing. Form data with bracketed keys is what Rails parses back into a nested
/// `position` hash.
async fn post_positioned(
    repo: &Repo,
    endpoint: &str,
    body: &str,
    anchor: &DiffAnchor,
    base_sha: &str,
    start_sha: &str,
    head_sha: &str,
) -> anyhow::Result<String> {
    let mut args: Vec<String> = vec![
        "api".into(),
        endpoint.to_string(),
        "--method".into(),
        "POST".into(),
        "--form".into(),
        format!("body={body}"),
        "--form".into(),
        "position[position_type]=text".into(),
        "--form".into(),
        format!("position[base_sha]={base_sha}"),
        "--form".into(),
        format!("position[start_sha]={start_sha}"),
        "--form".into(),
        format!("position[head_sha]={head_sha}"),
        "--form".into(),
        format!("position[old_path]={}", anchor.old_path),
        "--form".into(),
        format!("position[new_path]={}", anchor.new_path),
    ];
    if let Some(old) = anchor.old_line {
        args.push("--form".into());
        args.push(format!("position[old_line]={old}"));
    }
    if let Some(new) = anchor.new_line {
        args.push("--form".into());
        args.push(format!("position[new_line]={new}"));
    }
    glab_run(repo.local_path.clone(), args).await
}

/// Live MR fetch + DB upsert for GitLab repos.
pub(super) async fn fetch_and_upsert_mrs(
    wt: &Worktree,
    repo: &Repo,
    pool: &SqlitePool,
) -> anyhow::Result<Vec<Mr>> {
    let branch = wt.branch.clone();
    let out = glab_run(
        repo.local_path.clone(),
        vec![
            "mr".into(),
            "list".into(),
            "--source-branch".into(),
            branch,
            "--output".into(),
            "json".into(),
        ],
    )
    .await?;

    let items: Vec<serde_json::Value> = serde_json::from_str(&out).unwrap_or_default();
    let mut result = vec![];

    for item in items {
        let iid = item["iid"].as_u64().unwrap_or(0).to_string();
        let url = item["web_url"].as_str().unwrap_or("").to_string();
        let state = glab_state(item["state"].as_str().unwrap_or("opened"));

        let existing: Option<Mr> =
            sqlx::query_as("SELECT * FROM mrs WHERE worktree_id = ? AND remote_id = ?")
                .bind(&wt.id)
                .bind(&iid)
                .fetch_optional(pool)
                .await?;

        let mr = if let Some(existing_mr) = existing {
            sqlx::query("UPDATE mrs SET state = ?, url = ? WHERE id = ?")
                .bind(&state)
                .bind(&url)
                .bind(&existing_mr.id)
                .execute(pool)
                .await?;
            Mr { state, url, ..existing_mr }
        } else {
            let mr_id = uuid::Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT INTO mrs (id, worktree_id, platform, remote_id, url, state)
                 VALUES (?, ?, 'gitlab', ?, ?, ?)",
            )
            .bind(&mr_id)
            .bind(&wt.id)
            .bind(&iid)
            .bind(&url)
            .bind(&state)
            .execute(pool)
            .await?;
            Mr {
                id: mr_id,
                worktree_id: wt.id.clone(),
                platform: "gitlab".to_string(),
                remote_id: iid,
                url,
                state,
            }
        };

        result.push(mr);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The real hunk from wiremind/devops/cluster-manager!1828 that exposed this:
    /// commenting on new line 114 (a CONTEXT line, old 87) was rejected with
    /// `line_code: must be a valid line code` because only one number was sent.
    const HUNK: &str = concat!(
        "@@ -82,11 +106,14 @@ class GitlabSetCRPolicies(GitlabHelper):\n",
        "             logger.debug(f\"disabled\")\n",
        "             return\n",
        " \n",
        "+        # A project may define its own 'older_than'\n",
        "+        older_than = self.project_older_than.get(project.path_with_namespace)\n",
        "+\n",
        "         policy_attributes = {\n",
        "             \"cadence\": self.cadence,\n",
        "             \"enabled\": not self.disabled,\n",
    );

    #[test]
    fn context_line_carries_both_numbers() {
        // new 114 == old 87 in this hunk: header starts at old 82 / new 106, and
        // three added lines shift the two sides apart by 27.
        assert_eq!(resolve_in_diff(HUNK, 114), Ok((Some(87), Some(114))));
    }

    #[test]
    fn added_line_has_no_old_number() {
        // new 109-111 are the '+' lines.
        assert_eq!(resolve_in_diff(HUNK, 109), Ok((None, Some(109))));
    }

    #[test]
    fn deleted_line_is_found_on_the_old_side() {
        // Only new line 50 survives here, so 52 can only be the old-side deletion.
        let diff = "@@ -50,4 +50,1 @@\n ctx\n-gone1\n-gone2\n-gone3\n";
        assert_eq!(resolve_in_diff(diff, 52), Ok((Some(52), None)));
    }

    /// A number that is valid on BOTH sides must resolve as the new side — that is
    /// our convention for what an annotation's line means, and guessing otherwise
    /// would move comments to unrelated code.
    #[test]
    fn the_new_side_wins_when_a_number_exists_on_both() {
        let diff = "@@ -10,3 +10,2 @@\n ctx\n-gone\n ctx2\n";
        // new 11 is the context line `ctx2` (old 12); old 11 is the deleted `gone`.
        assert_eq!(resolve_in_diff(diff, 11), Ok((Some(12), Some(11))));
    }

    #[test]
    fn a_line_outside_the_diff_reports_what_is_commentable() {
        let commentable = resolve_in_diff(HUNK, 400).expect_err("400 is not in the hunk");
        assert_eq!(describe_ranges(&commentable), "106-114");
    }

    #[test]
    fn hunk_headers_parse_with_and_without_counts() {
        assert_eq!(parse_hunk_header("@@ -82,11 +106,14 @@ class X:"), Some((82, 106)));
        assert_eq!(parse_hunk_header("@@ -1 +1 @@"), Some((1, 1)));
        assert_eq!(parse_hunk_header(" not a header"), None);
    }

    #[test]
    fn ranges_collapse_into_readable_spans() {
        assert_eq!(describe_ranges(&[1, 2, 3, 9, 10, 40]), "1-3, 9-10, 40");
        assert_eq!(describe_ranges(&[]), "none");
    }
}
