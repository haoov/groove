use crate::core::db::models::Repo;

/// A forge CLI is not installed.
///
/// Typed rather than a message, because the caller has to TELL IT APART from a
/// failure: a machine with no `glab` is a machine with no GitLab repos, and the
/// review queue must stay quiet about it instead of showing an error. Raw io would
/// surface as "No such file or directory (os error 2)", localized, naming nothing.
#[derive(Debug)]
pub(super) struct CliMissing(pub &'static str);

impl std::fmt::Display for CliMissing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "`{}` is not installed, or not on the PATH this app was launched with",
            self.0
        )
    }
}
impl std::error::Error for CliMissing {}

/// True when the failure is only that the CLI is absent.
pub(super) fn is_cli_missing(e: &anyhow::Error) -> bool {
    e.downcast_ref::<CliMissing>().is_some()
}

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

/// The CLI owns auth on both platforms: `glab` for GitLab, `gh` for GitHub. There
/// is no token to store, which is why this cannot fail per-repo any more.
pub(super) fn make_client(repo: &Repo) -> Box<dyn PlatformClient> {
    if repo.host.contains("github") {
        Box::new(super::github::GhClient)
    } else {
        Box::new(super::gitlab::GlabClient)
    }
}
