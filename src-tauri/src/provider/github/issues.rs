//! The issue itself, over REST — body and state.

use crate::core::config::GithubConfig;
use crate::core::forge::api;

pub(super) async fn set_body(
    cfg: &GithubConfig,
    owner: &str,
    repo: &str,
    number: i64,
    markdown: &str,
) -> anyhow::Result<()> {
    api::github(
        &cfg.host,
        reqwest::Method::PATCH,
        &format!("repos/{owner}/{repo}/issues/{number}"),
        Some(&serde_json::json!({ "body": markdown })),
    )
    .await?;
    Ok(())
}

/// Close an issue as not planned — the discard path, which is not the same as
/// finishing it. Finishing sets the board's Status; this says it will not happen.
pub(super) async fn close_not_planned(
    cfg: &GithubConfig,
    owner: &str,
    repo: &str,
    number: i64,
) -> anyhow::Result<()> {
    api::github(
        &cfg.host,
        reqwest::Method::PATCH,
        &format!("repos/{owner}/{repo}/issues/{number}"),
        Some(&serde_json::json!({ "state": "closed", "state_reason": "not_planned" })),
    )
    .await?;
    Ok(())
}

const ADD_TO_BOARD: &str = r#"
mutation($project: ID!, $content: ID!) {
  addProjectV2ItemById(input: { projectId: $project, contentId: $content }) {
    item { id }
  }
}
"#;

/// File an issue and put it on the board. Returns `(number, url, node_id)`.
pub(super) async fn create(
    cfg: &GithubConfig,
    owner: &str,
    repo: &str,
    title: &str,
    body: &str,
) -> anyhow::Result<(i64, String, String)> {
    let created = api::github(
        &cfg.host,
        reqwest::Method::POST,
        &format!("repos/{owner}/{repo}/issues"),
        Some(&serde_json::json!({ "title": title, "body": body })),
    )
    .await?;

    let number = created["number"]
        .as_i64()
        .ok_or_else(|| anyhow::anyhow!("GitHub returned an issue with no number"))?;
    let url = created["html_url"].as_str().unwrap_or_default().to_string();
    let node_id = created["node_id"].as_str().unwrap_or_default().to_string();
    Ok((number, url, node_id))
}

/// Put an existing issue on a board. Idempotent — GitHub returns the existing
/// item when it is already there.
pub(super) async fn add_to_board(
    cfg: &GithubConfig,
    project_id: &str,
    content_node_id: &str,
) -> anyhow::Result<String> {
    let res = api::github_graphql(
        &cfg.host,
        ADD_TO_BOARD,
        serde_json::json!({ "project": project_id, "content": content_node_id }),
    )
    .await?;
    Ok(res["data"]["addProjectV2ItemById"]["item"]["id"]
        .as_str()
        .unwrap_or_default()
        .to_string())
}
