use serde::Serialize;

use crate::core::db::models::Repo;

/// MR/PR fields for the overview page, normalized across platforms.
#[derive(Debug, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct MrDetails {
    pub title: String,
    pub description: String,
    pub author: String,
    pub source_branch: String,
    pub target_branch: String,
    /// "open" | "merged" | "closed".
    pub state: String,
    pub draft: bool,
    pub created_at: String,
    pub web_url: String,
    /// Folded in from the approvals endpoint; None when that call failed.
    pub approval: Option<MrApproval>,
}

/// The MR/PR's approval state.
#[derive(Debug, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct MrApproval {
    pub approved: bool,
    /// The token's own user is among the approvers.
    pub approved_by_me: bool,
    pub approved_by: Vec<String>,
}

/// The MR/PR pipeline: one status word plus the run's URL.
#[derive(Debug, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct CiStatus {
    pub status: String,
    pub url: String,
}

/// One MR/PR discussion thread. `notes` is never empty.
#[derive(Debug, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct MrThread {
    pub id: String,
    pub notes: Vec<MrNote>,
}

/// One note in a thread.
#[derive(Debug, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct MrNote {
    pub author: String,
    pub body: String,
    pub created_at: String,
    pub resolved: bool,
    /// False for a plain conversation comment, which no forge resolves.
    pub resolvable: bool,
    /// None for a note that is not anchored in the diff.
    pub position: Option<NotePosition>,
}

/// Where a note is anchored in the MR/PR's diff.
#[derive(Debug, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct NotePosition {
    pub new_path: Option<String>,
    #[ts(type = "number | null")]
    pub new_line: Option<i64>,
    /// GitLab only: GitHub keeps no position for an old-side note.
    pub old_path: Option<String>,
    #[ts(type = "number | null")]
    pub old_line: Option<i64>,
    /// GitLab only: the last line of a multi-line note's range.
    #[ts(type = "number | null")]
    pub end_new_line: Option<i64>,
}

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
    /// MR/PR fields for the overview page. `approval` is filled by `forge::commands`.
    async fn get_mr_details(&self, repo: &Repo, remote_id: &str) -> anyhow::Result<MrDetails>;
    async fn get_mr_threads(&self, repo: &Repo, remote_id: &str) -> anyhow::Result<Vec<MrThread>>;
    /// None when the MR has no pipeline.
    async fn get_mr_ci(&self, repo: &Repo, remote_id: &str) -> anyhow::Result<Option<CiStatus>>;
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
    async fn get_mr_approval(&self, repo: &Repo, remote_id: &str) -> anyhow::Result<MrApproval>;
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
    match Forge::of_host(&repo.host) {
        Forge::Github => Box::new(super::github::GhClient),
        Forge::Gitlab => Box::new(super::gitlab::GlabClient),
    }
}

/// Which forge hosts a repo. The one place that reads a host name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Forge {
    Github,
    Gitlab,
}

impl Forge {
    pub(crate) fn of_host(host: &str) -> Self {
        if host.contains("github") {
            Self::Github
        } else {
            Self::Gitlab
        }
    }

    /// The name stored on an MR row and read by the frontend sigil.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Github => "github",
            Self::Gitlab => "gitlab",
        }
    }

    /// The forge of a merge-request web URL.
    pub(crate) fn of_url(url: &str) -> Self {
        let host = url
            .split_once("://")
            .map(|(_, rest)| rest)
            .unwrap_or(url)
            .split('/')
            .next()
            .unwrap_or("");
        Self::of_host(host)
    }
}

#[cfg(test)]
mod tests {
    use super::Forge;

    #[test]
    fn a_host_names_its_forge() {
        assert_eq!(Forge::of_host("github.com"), Forge::Github);
        assert_eq!(Forge::of_host("github.acme.dev"), Forge::Github);
        assert_eq!(Forge::of_host("gitlab.example.com"), Forge::Gitlab);
        assert_eq!(Forge::of_host("git.internal"), Forge::Gitlab);
        assert_eq!(Forge::Github.as_str(), "github");
        assert_eq!(Forge::Gitlab.as_str(), "gitlab");
    }

    /// A path segment must never decide the forge: only the host does.
    #[test]
    fn a_url_names_its_forge_by_host_only() {
        assert_eq!(
            Forge::of_url("https://github.com/o/r/pull/4"),
            Forge::Github
        );
        assert_eq!(
            Forge::of_url("https://gitlab.example.com/g/p/-/merge_requests/7"),
            Forge::Gitlab
        );
        assert_eq!(
            Forge::of_url("https://gitlab.example.com/team/github-actions/-/merge_requests/1"),
            Forge::Gitlab,
            "a repo named github does not make it a GitHub MR"
        );
    }
}
