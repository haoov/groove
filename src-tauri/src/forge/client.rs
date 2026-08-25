use crate::core::db::models::Repo;

#[async_trait::async_trait]
pub(super) trait PlatformClient: Send + Sync {
    fn platform_name(&self) -> &'static str;
    async fn create_mr(
        &self,
        repo: &Repo,
        branch: &str,
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
    /// Rich MR/PR fields for the overview page, normalized across platforms:
    /// `{ title, description, author, source_branch, target_branch, state,
    ///    draft, created_at, web_url }`.
    async fn get_mr_details(&self, repo: &Repo, remote_id: &str)
        -> anyhow::Result<serde_json::Value>;
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
    /// `{ approved, approved_by_me, approved_by: [username] }` — whether the MR
    /// carries an approval at all, and specifically one from the current user.
    async fn get_mr_approval(&self, repo: &Repo, remote_id: &str) -> anyhow::Result<serde_json::Value>;
    /// Post a comment on the MR/PR: a general note, or — when `position` is
    /// `(new_path, new_line)` — a discussion anchored to that line of the MR
    /// head's diff.
    async fn post_mr_comment(
        &self,
        repo: &Repo,
        remote_id: &str,
        body: &str,
        position: Option<(&str, i64)>,
    ) -> anyhow::Result<()>;
}

/// The API layer owns the calls; the CLIs only supply tokens (see `auth`).
pub(super) fn make_client(repo: &Repo) -> Box<dyn PlatformClient> {
    if repo.host.contains("github") {
        Box::new(super::github::GhClient)
    } else {
        Box::new(super::gitlab::GlabClient)
    }
}
