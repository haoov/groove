use crate::{Error, MrId, Result, Timestamp, WorktreeId};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Forge {
    Gitlab,
    Github,
}

impl Forge {
    pub fn as_str(self) -> &'static str {
        match self {
            Forge::Gitlab => "gitlab",
            Forge::Github => "github",
        }
    }

    /// The mark its forge puts before a merge request number.
    pub fn sigil(self) -> char {
        match self {
            Forge::Github => '#',
            Forge::Gitlab => '!',
        }
    }

    /// The forge a host names; nothing but the host decides it.
    pub fn of_host(host: &str) -> Self {
        match host.contains("github") {
            true => Forge::Github,
            false => Forge::Gitlab,
        }
    }

    /// The GraphQL endpoint of a host. One carrying its own scheme stands as it is.
    pub fn graphql(host: &str) -> String {
        match host {
            "github.com" => "https://api.github.com/graphql".to_string(),
            host if host.starts_with("http") => format!("{host}/api/graphql"),
            host => format!("https://{host}/api/graphql"),
        }
    }

    pub fn parse(name: &str) -> Result<Self> {
        match name {
            "gitlab" => Ok(Forge::Gitlab),
            "github" => Ok(Forge::Github),
            _ => Err(Error::invalid(format!("unknown forge {name}"))),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MrState {
    Open,
    Merged,
    Closed,
}

impl MrState {
    pub fn label(self) -> &'static str {
        match self {
            MrState::Open => "open",
            MrState::Merged => "merged",
            MrState::Closed => "closed",
        }
    }
}

/// The one MR a worktree may have.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct Mr {
    pub id: MrId,
    pub worktree: WorktreeId,
    pub forge: Forge,
    pub remote_id: String,
    pub url: String,
    pub state: MrState,
}

#[derive(Clone, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MrApproval {
    pub approved: bool,
    pub approved_by_me: bool,
    pub approved_by: Vec<String>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewState {
    Requested,
    Approved,
    ChangesRequested,
    Commented,
}

#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct Reviewer {
    pub name: String,
    pub state: ReviewState,
    pub at: Option<Timestamp>,
}

/// MR fields for the overview, the same shape on both forges.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct MrDetails {
    pub title: String,
    pub description: String,
    pub author: String,
    pub source_branch: String,
    pub target_branch: String,
    pub state: MrState,
    pub draft: bool,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub web_url: String,
    pub approval: Option<MrApproval>,
    pub reviewers: Vec<Reviewer>,
}

impl MrDetails {
    pub fn changes_requested(&self) -> bool {
        self.reviewers
            .iter()
            .any(|r| r.state == ReviewState::ChangesRequested)
    }

    /// The most pressing verdict given, then approval, then a review still awaited.
    pub fn review(&self) -> Option<ReviewState> {
        let has = |state| self.reviewers.iter().any(|one| one.state == state);
        let approved = self.approval.as_ref().is_some_and(|one| one.approved);
        match () {
            () if has(ReviewState::ChangesRequested) => Some(ReviewState::ChangesRequested),
            () if has(ReviewState::Commented) => Some(ReviewState::Commented),
            () if approved => Some(ReviewState::Approved),
            () => has(ReviewState::Requested).then_some(ReviewState::Requested),
        }
    }

    pub fn review_requested_at(&self) -> Option<Timestamp> {
        self.at_of(ReviewState::Requested).min()
    }

    /// When a reviewer first asked for changes.
    pub fn changes_requested_at(&self) -> Option<Timestamp> {
        self.at_of(ReviewState::ChangesRequested).min()
    }

    /// When the last approval landed, and only while it stands approved.
    pub fn approved_at(&self) -> Option<Timestamp> {
        let approved = self.approval.as_ref().is_some_and(|one| one.approved);
        approved
            .then(|| self.at_of(ReviewState::Approved).max())
            .flatten()
    }

    fn at_of(&self, state: ReviewState) -> impl Iterator<Item = Timestamp> + '_ {
        self.reviewers
            .iter()
            .filter(move |one| one.state == state)
            .filter_map(|one| one.at)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CiState {
    Pending,
    Running,
    Success,
    Failed,
    Canceled,
    Skipped,
    Unknown,
}

impl CiState {
    /// The one word a row shows for it.
    pub fn label(self) -> &'static str {
        match self {
            CiState::Pending => "pending",
            CiState::Running => "running",
            CiState::Success => "passed",
            CiState::Failed => "failed",
            CiState::Canceled => "canceled",
            CiState::Skipped => "skipped",
            CiState::Unknown => "unknown",
        }
    }

    /// The state that wins when two checks disagree.
    pub fn worst(self, other: Self) -> Self {
        match Self::rank(other) > Self::rank(self) {
            true => other,
            false => self,
        }
    }

    /// How much a state is worth showing; the higher wins.
    fn rank(state: Self) -> u8 {
        match state {
            CiState::Failed => 6,
            CiState::Running => 5,
            CiState::Pending => 4,
            CiState::Canceled => 3,
            CiState::Unknown => 2,
            CiState::Success => 1,
            CiState::Skipped => 0,
        }
    }

    pub fn is_green(self) -> bool {
        self == CiState::Success
    }
}

#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct CiStatus {
    pub state: CiState,
    pub url: String,
    pub finished_at: Option<Timestamp>,
}

/// One discussion thread. `notes` is never empty.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct MrThread {
    pub id: String,
    pub notes: Vec<MrNote>,
}

#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct MrNote {
    pub author: String,
    pub body: String,
    pub created_at: Timestamp,
    pub resolved: bool,
    pub resolvable: bool,
    pub position: Option<NotePosition>,
}

/// Where a note is anchored in the diff.
#[derive(Clone, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NotePosition {
    pub new_path: Option<String>,
    pub new_line: Option<u32>,
    pub old_path: Option<String>,
    pub old_line: Option<u32>,
    pub end_new_line: Option<u32>,
}

/// An open MR where the user is a requested reviewer.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct ReviewMr {
    pub forge: Forge,
    pub project: String,
    pub iid: u64,
    pub title: String,
    pub author: String,
    pub source_branch: String,
    pub target_branch: String,
    pub draft: bool,
    pub web_url: String,
    pub updated_at: Timestamp,
    pub local_path: Option<String>,
    pub approved: bool,
}

impl ReviewMr {
    /// The session that reviews it, the same one every time it is opened.
    pub fn session_id(&self) -> String {
        let project = self
            .project
            .chars()
            .map(|c| match c.is_ascii_alphanumeric() {
                true => c.to_ascii_lowercase(),
                false => '-',
            })
            .collect::<String>();
        format!("review-{project}-{}", self.iid)
    }

    /// Where its repo is cloned from, read off the page the MR stands on.
    pub fn clone_url(&self) -> Option<String> {
        let at = self
            .web_url
            .find("/-/merge_requests/")
            .or_else(|| self.web_url.find("/pull/"))?;
        let repo = self.web_url.get(..at)?;
        Some(format!("{repo}.git"))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewVerdict {
    Approve,
    RequestChanges,
    Comment,
}
