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
