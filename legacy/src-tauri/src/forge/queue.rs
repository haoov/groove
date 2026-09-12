//! The review queue: every open MR/PR where the current user is a requested
//! reviewer, across every forge host present in the clone pool.

use crate::core::error::{AppError, AppResult, ErrorKind};
use crate::core::forge::api::pct;

/// An open MR where the current user is a reviewer, matched by pool slug to its MAIN clone.
#[derive(Debug, Clone, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct ReviewMr {
    /// "gitlab" or "github".
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
    /// Approved by anyone but not merged.
    pub approved: bool,
}

/// The pool's hosts, and the clone path of every slug. A pool slug is `<host>/<group…>/<project>`.
fn clone_index(
    repos: &[crate::worktrees::MainRepo],
) -> (
    std::collections::BTreeSet<String>,
    std::collections::HashMap<String, String>,
) {
    let mut hosts = std::collections::BTreeSet::new();
    let mut clone_by_project = std::collections::HashMap::new();
    for r in repos {
        let Some((host, _)) = r.slug.split_once('/') else {
            continue;
        };
        hosts.insert(host.to_string());
        clone_by_project.insert(r.slug.clone(), r.local_path.clone());
    }
    (hosts, clone_by_project)
}

/// Merge the per-host answers: an absent CLI is skipped, and an error surfaces only when
/// no host produced a row. Newest first.
fn merge_reviews(
    results: Vec<(String, anyhow::Result<Vec<ReviewMr>>)>,
) -> AppResult<Vec<ReviewMr>> {
    let mut out = vec![];
    let mut errors = vec![];
    for (host, result) in results {
        match result {
            Ok(mut mrs) => out.append(&mut mrs),
            Err(e) if crate::core::forge::auth::is_cli_missing(&e) => {
                tracing::debug!("{host} review queue skipped: {e}");
            }
            Err(e) => errors.push(format!("{host}: {e}")),
        }
    }
    if out.is_empty() && !errors.is_empty() {
        return Err(AppError::new(ErrorKind::Forge, errors.join(" · ")));
    }
    out.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(out)
}

/// Ask every forge host in the pool; one host failing does not blank the others.
#[tauri::command]
pub async fn list_review_mrs() -> AppResult<Vec<ReviewMr>> {
    let main_repos = crate::worktrees::list_main_repos().await?;
    let (hosts, clone_by_project) = clone_index(&main_repos);
    if hosts.is_empty() {
        return Err(AppError::conflict("no repos in the pool — clone one first"));
    }

    let results = futures_util::future::join_all(hosts.iter().map(|host| {
        let clones = &clone_by_project;
        async move {
            let mrs = match super::client::Forge::of_host(host) {
                super::client::Forge::Github => github_reviews(host, clones).await,
                super::client::Forge::Gitlab => gitlab_reviews(host, clones).await,
            };
            (host.clone(), mrs)
        }
    }))
    .await;

    merge_reviews(results)
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
                local_path: clone_by_project
                    .get(&format!("{host}/{project_full}"))
                    .cloned(),
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
            local_path: clone_by_project
                .get(&format!("{host}/{project_full}"))
                .cloned(),
            iid: item["iid"].as_u64().unwrap_or(0),
            title: item["title"].as_str().unwrap_or("").to_string(),
            author: item["author"]["username"]
                .as_str()
                .or(item["author"]["name"].as_str())
                .unwrap_or("")
                .to_string(),
            source_branch: item["source_branch"].as_str().unwrap_or("").to_string(),
            target_branch: item["target_branch"].as_str().unwrap_or("").to_string(),
            draft: item["draft"]
                .as_bool()
                .or(item["work_in_progress"].as_bool())
                .unwrap_or(false),
            web_url: item["web_url"].as_str().unwrap_or("").to_string(),
            updated_at: item["updated_at"].as_str().unwrap_or("").to_string(),
            // Filled in below.
            approved: false,
            platform: "gitlab".into(),
            project_full,
        });
    }

    // Approval is not in the list payload; one concurrent call per MR.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worktrees::MainRepo;

    fn repo(slug: &str) -> MainRepo {
        MainRepo {
            local_path: format!("/pool/{slug}"),
            slug: slug.to_string(),
        }
    }

    fn mr(host: &str, project: &str, iid: u64, updated_at: &str) -> ReviewMr {
        ReviewMr {
            platform: "gitlab".into(),
            project_full: project.into(),
            iid,
            title: format!("MR {iid}"),
            author: "someone".into(),
            source_branch: "feat/x".into(),
            target_branch: "main".into(),
            draft: false,
            web_url: format!("https://{host}/{project}/-/merge_requests/{iid}"),
            updated_at: updated_at.into(),
            local_path: None,
            approved: false,
        }
    }

    #[test]
    fn the_pool_index_keys_a_clone_by_its_slug() {
        let repos = vec![
            repo("gitlab.example.com/group/api"),
            repo("github.com/owner/tool"),
            repo("no-slash"),
        ];
        let (hosts, clones) = clone_index(&repos);
        assert_eq!(
            hosts.iter().map(String::as_str).collect::<Vec<_>>(),
            vec!["github.com", "gitlab.example.com"],
            "hosts are unique and ordered"
        );
        assert_eq!(
            clones
                .get("gitlab.example.com/group/api")
                .map(String::as_str),
            Some("/pool/gitlab.example.com/group/api")
        );
        assert!(
            !clones.contains_key("no-slash"),
            "a slug with no host is skipped"
        );
    }

    #[test]
    fn merged_rows_are_newest_first() {
        let merged = merge_reviews(vec![
            (
                "a".into(),
                Ok(vec![mr("a", "g/p", 1, "2026-01-01T00:00:00Z")]),
            ),
            (
                "b".into(),
                Ok(vec![
                    mr("b", "g/q", 2, "2026-03-01T00:00:00Z"),
                    mr("b", "g/r", 3, "2026-02-01T00:00:00Z"),
                ]),
            ),
        ])
        .unwrap();
        assert_eq!(
            merged.iter().map(|m| m.iid).collect::<Vec<_>>(),
            vec![2, 3, 1]
        );
    }

    #[test]
    fn an_absent_cli_is_not_a_failure() {
        let missing = anyhow::Error::new(crate::core::forge::auth::CliMissing("glab"));
        let merged = merge_reviews(vec![
            ("gitlab.example.com".into(), Err(missing)),
            (
                "github.com".into(),
                Ok(vec![mr("github.com", "o/t", 7, "2026-01-01T00:00:00Z")]),
            ),
        ])
        .unwrap();
        assert_eq!(merged.len(), 1, "the reachable host still answers");
    }

    /// One host failing while another answers must not hide the rows that arrived.
    #[test]
    fn a_partial_failure_keeps_what_arrived() {
        let merged = merge_reviews(vec![
            ("gitlab.example.com".into(), Err(anyhow::anyhow!("500"))),
            (
                "github.com".into(),
                Ok(vec![mr("github.com", "o/t", 7, "2026-01-01T00:00:00Z")]),
            ),
        ])
        .unwrap();
        assert_eq!(merged.len(), 1);
    }

    #[test]
    fn every_host_failing_is_reported() {
        let err = merge_reviews(vec![
            ("a".into(), Err(anyhow::anyhow!("boom"))),
            ("b".into(), Err(anyhow::anyhow!("bang"))),
        ])
        .unwrap_err();
        assert_eq!(err.kind, ErrorKind::Forge);
        assert!(err.message.contains("a: boom"), "{err}");
        assert!(err.message.contains("b: bang"), "{err}");
    }

    #[test]
    fn no_reviews_anywhere_is_an_empty_list() {
        assert!(merge_reviews(vec![("a".into(), Ok(vec![]))])
            .unwrap()
            .is_empty());
    }
}
