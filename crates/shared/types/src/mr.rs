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

    pub fn review_requested_at(&self) -> Option<Timestamp> {
        self.reviewers
            .iter()
            .filter(|r| r.state == ReviewState::Requested)
            .filter_map(|r| r.at)
            .min()
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
    pub fn is_green(self) -> bool {
        self == CiState::Success
    }

    pub fn is_red(self) -> bool {
        self == CiState::Failed
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

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewVerdict {
    Approve,
    RequestChanges,
    Comment,
}
