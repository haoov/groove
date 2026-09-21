//! Data and pure rules. No IO, no async, nothing with a side effect.

mod activity;
mod annotation;
mod approval;
mod attention;
mod config;
mod delivery;
mod diff;
mod editing;
mod error;
mod ids;
mod mr;
mod naming;
mod narrowing;
mod note;
mod panes;
mod repo;
mod session;
mod syntax;
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
    Config, ConfigView, FilterConfig, GitConfig, GithubConfig, GithubView, NotionConfig,
    NotionView, Preferences, PriorityMap, PropertyNames, StatusMap, ThemeName, UiConfig,
};
pub use delivery::{MrDelivery, WorktreeDelivery};
pub use diff::{
    BlameLine, CommitEntry, DiffMode, DiffView, FileDiff, FileStatus, LineMark, RepoDiff, Row,
    RowKind, word_diff_pairs,
};
pub use editing::{Caret, Edit, Indent, Motion, Selection};
pub use error::{Error, ErrorKind, Result};
pub use ids::{
    AnnotationId, ApprovalId, ExternalId, Generation, MrId, RepoId, SessionId, WorktreeId,
};
pub use mr::{
    CiState, CiStatus, Forge, Mr, MrApproval, MrDetails, MrNote, MrState, MrThread, NotePosition,
    ReviewMr, ReviewState, ReviewVerdict, Reviewer,
};
pub use naming::names_session;
pub use narrowing::{narrows, score};
pub use note::{Anchor, Note, NoteOrigin, Said};
pub use panes::Panes;
pub use repo::{PoolEntry, Repo, Worktree, WorktreeSpec, WorktreeStatus};
pub use session::{Session, SessionKind, SessionState};
pub use syntax::{Capture, Highlight};
pub use task::{
    Priority, ProviderId, Span, StatusIntent, Task, TaskDates, TaskKey, TimeSummary, hours,
};
pub use terminal::{AnsiPalette, Rgb, Screen, ScreenCell};
pub use time::{Day, Timestamp};
pub use timeline::{TimelineEvent, TimelineKind};
