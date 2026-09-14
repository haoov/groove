//! Data and pure rules. No IO, no async, nothing with a side effect.

mod activity;
mod annotation;
mod approval;
mod attention;
mod config;
mod delivery;
mod diff;
mod error;
mod ids;
mod mr;
mod naming;
mod repo;
mod schema;
mod session;
mod task;
mod terminal;
mod time;
mod timeline;

#[cfg(test)]
mod tests;

pub use activity::{AgentStatus, Ask, AttentionClass, HookKind, SessionActivity, ToolCall};
pub use annotation::{Annotation, AnnotationStatus};
pub use approval::{Approval, Decision, Origin};
pub use attention::{Attention, MrFacts, Thresholds, attention};
pub use config::{
    Config, ConfigView, FilterConfig, GitConfig, GithubConfig, GithubPropertyNames, NotionConfig,
    NotionView, Preferences, PropertyNames, StatusMap, ThemeName, UiConfig,
};
pub use delivery::{MrDelivery, WorktreeDelivery};
pub use diff::{
    BlameLine, CommitEntry, DiffLine, DiffMode, DiffView, FileDiff, FileStatus, Hunk, LineKind,
    RepoDiff, word_diff_pairs,
};
pub use error::{Error, ErrorKind, Result};
pub use ids::{
    AnnotationId, ApprovalId, ExternalId, Generation, MrId, RepoId, SessionId, WorktreeId,
};
pub use mr::{
    CiState, CiStatus, Forge, Mr, MrApproval, MrDetails, MrNote, MrState, MrThread, NotePosition,
    ReviewMr, ReviewState, ReviewVerdict, Reviewer,
};
pub use naming::names_session;
pub use repo::{PoolEntry, Repo, Worktree, WorktreeSpec, WorktreeStatus};
pub use schema::{
    PropertyKind, PropertyOption, PropertySchema, PropertyValue, StatusGroup, TaskSchema,
};
pub use session::{Session, SessionKind, SessionState};
pub use task::{ProviderId, Span, StatusIntent, Task, TaskDates, TaskKey, TimeSummary};
pub use terminal::{AnsiPalette, Rgb, Screen, ScreenCell};
pub use time::{Day, Timestamp};
pub use timeline::{TimelineEvent, TimelineKind};
