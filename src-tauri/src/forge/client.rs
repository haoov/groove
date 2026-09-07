use crate::core::db::models::Repo;

#[async_trait::async_trait]
pub(super) trait PlatformClient: Send + Sync {
    fn platform_name(&self) -> &'static str;
    /// `target` is the branch the MR lands on, resolved by `forge::ops`.
    async fn create_mr(
        &self,
        repo: &Repo,
        branch: &str,
        target: &str,
        title: &str,
        description: &str,
    ) -> anyhow::Result<(String, String)>;
    async fn update_mr(
        &self,
        repo: &Repo,
        remote_id: &str,
        title: Option<&str>,
        description: Option<&str>,
    ) -> anyhow::Result<()>;
    async fn close_mr(&self, repo: &Repo, remote_id: &str) -> anyhow::Result<()>;
    /// MR/PR fields for the overview page, normalized across platforms.
    async fn get_mr_details(
        &self,
        repo: &Repo,
        remote_id: &str,
    ) -> anyhow::Result<serde_json::Value>;
    /// Review threads in the UI's shape: `[{ id, notes: [...] }]`.
    async fn get_mr_threads(
        &self,
        repo: &Repo,
        remote_id: &str,
    ) -> anyhow::Result<serde_json::Value>;
    /// Latest CI/pipeline status for the MR — `{ "status": str, "url": str }` or Null.
    async fn get_mr_ci(&self, repo: &Repo, remote_id: &str) -> anyhow::Result<serde_json::Value>;
    async fn reply_to_thread(
        &self,
        repo: &Repo,
        remote_id: &str,
        thread_id: &str,
        body: &str,
    ) -> anyhow::Result<()>;
    async fn resolve_mr_thread(
        &self,
        repo: &Repo,
        remote_id: &str,
        thread_id: &str,
    ) -> anyhow::Result<()>;
    /// Approve the MR/PR as the current user.
    async fn approve_mr(&self, repo: &Repo, remote_id: &str) -> anyhow::Result<()>;
    /// `{ approved, approved_by_me, approved_by: [username] }`.
    async fn get_mr_approval(
        &self,
        repo: &Repo,
        remote_id: &str,
    ) -> anyhow::Result<serde_json::Value>;
    /// Post a general note, or a discussion anchored at `position = (new_path, new_line)` of the MR head's diff.
    async fn post_mr_comment(
        &self,
        repo: &Repo,
        remote_id: &str,
        body: &str,
        position: Option<(&str, i64)>,
    ) -> anyhow::Result<()>;
}

/// The API client for the repo's host.
pub(super) fn make_client(repo: &Repo) -> Box<dyn PlatformClient> {
    if repo.host.contains("github") {
        Box::new(super::github::GhClient)
    } else {
        Box::new(super::gitlab::GlabClient)
    }
}
