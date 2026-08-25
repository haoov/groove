//! The review queue: every open MR/PR where the current user is a requested
//! reviewer, across every forge host present in the clone pool.

use crate::core::forge::api::pct;

/// An open MR where the current user is a reviewer, matched (by pool slug) to
/// its MAIN clone when one exists. TS mirror in types/ipc.ts.
#[derive(Debug, Clone, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct ReviewMr {
    /// Which forge this row came from: "gitlab" or "github". The UI reads it for
    /// the reference sigil (`!42` vs `#42`).
    pub platform: String,
    /// Full project path on the forge, e.g. "wiremind/devops/gitlab-ci-common".
    pub project_full: String,
    #[ts(type = "number")]
    pub iid: u64,
    pub title: String,
    pub author: String,
    pub source_branch: String,
    pub target_branch: String,
    pub draft: bool,
    pub web_url: String,
    pub updated_at: String,
    /// MAIN clone path when the project is already cloned locally.
    pub local_path: Option<String>,
    /// Already approved (by anyone) but not merged — shown as a pill in Up next.
    pub approved: bool,
}

/// Every forge host in the pool is asked, so a self-managed GitLab and
/// gitlab.com both report. One host failing (CLI missing, not logged in) must
/// not blank the others: each is best-effort and only a total failure surfaces.
#[tauri::command]
pub async fn list_review_mrs() -> Result<Vec<ReviewMr>, String> {
    let main_repos = crate::worktrees::list_main_repos().await.map_err(|e| e.to_string())?;

    // The pool layout is the identity: slug = <host>/<group…>/<project>.
    let mut hosts: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut clone_by_project: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    for r in &main_repos {
        let Some((host, _)) = r.slug.split_once('/') else { continue };
        hosts.insert(host.to_string());
        clone_by_project.insert(r.slug.clone(), r.local_path.clone());
    }
    if hosts.is_empty() {
        return Err("no repos in the pool — clone one first".to_string());
    }

    let results = futures_util::future::join_all(hosts.iter().map(|host| {
        let clones = &clone_by_project;
        async move {
            // Same rule as client::make_client, so the queue and the MR views
            // cannot disagree about which forge a host is.
            let mrs = if host.contains("github") {
                github_reviews(host, clones).await
            } else {
                gitlab_reviews(host, clones).await
            };
            (host.clone(), mrs)
        }
    }))
    .await;

    let mut out = vec![];
    let mut errors = vec![];
    for (host, result) in results {
        match result {
            Ok(mut mrs) => out.append(&mut mrs),
            // No CLI for that forge means no repos on it — not something to report.
            Err(e) if crate::core::forge::auth::is_cli_missing(&e) => {
                tracing::debug!("{host} review queue skipped: {e}");
            }
            Err(e) => errors.push(format!("{host}: {e}")),
        }
    }
    if out.is_empty() && !errors.is_empty() {
        return Err(errors.join(" · "));
    }
    // Most recently touched first, across every host.
    out.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(out)
}

async fn github_reviews(
    host: &str,
    clone_by_project: &std::collections::HashMap<String, String>,
) -> anyhow::Result<Vec<ReviewMr>> {
    let nodes = super::github::review_requested_prs(host).await?;
    Ok(nodes
        .iter()
        .filter_map(|pr| {
            let project_full = pr["repository"]["nameWithOwner"].as_str()?.to_string();
            Some(ReviewMr {
                local_path: clone_by_project.get(&format!("{host}/{project_full}")).cloned(),
                iid: pr["number"].as_u64()?,
                title: pr["title"].as_str().unwrap_or("").to_string(),
                author: pr["author"]["login"].as_str().unwrap_or("").to_string(),
                source_branch: pr["headRefName"].as_str().unwrap_or("").to_string(),
                target_branch: pr["baseRefName"].as_str().unwrap_or("").to_string(),
                draft: pr["isDraft"].as_bool().unwrap_or(false),
                web_url: pr["url"].as_str().unwrap_or("").to_string(),
                updated_at: pr["updatedAt"].as_str().unwrap_or("").to_string(),
                approved: pr["reviewDecision"].as_str() == Some("APPROVED"),
                platform: "github".into(),
                project_full,
            })
        })
        .collect())
}

async fn gitlab_reviews(
    host: &str,
    clone_by_project: &std::collections::HashMap<String, String>,
) -> anyhow::Result<Vec<ReviewMr>> {
    let (_, username) = super::gitlab::current_user(host).await?;

    let path = format!(
        "merge_requests?reviewer_username={}&state=opened&scope=all&per_page=50",
        pct(&username)
    );
    let v = crate::core::forge::api::gitlab(host, reqwest::Method::GET, &path, None).await?;
    let items = v.as_array().cloned().unwrap_or_default();

    let mut result = vec![];
    for item in items {
        // references.full = "<group/project>!<iid>"
        let full = item["references"]["full"].as_str().unwrap_or("");
        let project_full = full.split('!').next().unwrap_or("").to_string();
        if project_full.is_empty() {
            continue;
        }
        result.push(ReviewMr {
            local_path: clone_by_project.get(&format!("{host}/{project_full}")).cloned(),
            iid: item["iid"].as_u64().unwrap_or(0),
            title: item["title"].as_str().unwrap_or("").to_string(),
            author: item["author"]["username"]
                .as_str()
                .or(item["author"]["name"].as_str())
                .unwrap_or("")
                .to_string(),
            source_branch: item["source_branch"].as_str().unwrap_or("").to_string(),
            target_branch: item["target_branch"].as_str().unwrap_or("").to_string(),
            draft: item["draft"].as_bool().or(item["work_in_progress"].as_bool()).unwrap_or(false),
            web_url: item["web_url"].as_str().unwrap_or("").to_string(),
            updated_at: item["updated_at"].as_str().unwrap_or("").to_string(),
            // Filled in concurrently below — one approvals call per MR.
            approved: false,
            platform: "gitlab".into(),
            project_full,
        });
    }

    // Approval isn't in the list payload, so fetch it per MR. Concurrent, so the
    // queue costs one extra round-trip in wall time rather than N.
    let flags = futures_util::future::join_all(
        result
            .iter()
            .map(|mr| super::gitlab::mr_approved(host, &mr.project_full, mr.iid)),
    )
    .await;
    for (mr, approved) in result.iter_mut().zip(flags) {
        mr.approved = approved;
    }

    Ok(result)
}
