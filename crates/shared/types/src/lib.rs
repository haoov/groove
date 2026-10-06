//! Data and pure rules. No IO, no async, nothing with a side effect.

mod activity;
mod annotation;
mod approval;
mod attention;
mod chord;
mod cluster;
mod config;
mod delivery;
mod diff;
mod editing;
mod environment;
mod error;
pub mod front_matter;
mod ids;
pub mod json;
mod mapping;
mod mr;
mod naming;
mod narrowing;
mod note;
mod panes;
mod repo;
mod review;
mod routine;
mod secret;
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
pub use attention::{Attention, MrFacts, Thresholds, attention, starts_today};
pub use chord::{Chord, Stroke};
pub use cluster::{KubeAuth, KubeContext, Login};
pub use config::{
    Config, ConfigView, EstimateUnit, FilterConfig, FontFamily, GitConfig, GithubConfig,
    GithubView, NotionConfig, NotionView, Preferences, PriorityMap, PropertyNames, REDACTED,
    RoutinesConfig, SharedConfig, StatusMap, ThemeName, UiConfig,
};
pub use delivery::{MrDelivery, Standing, Step, WorktreeDelivery};
pub use diff::{
    BlameLine, CommitEntry, DiffMode, DiffView, FileDiff, FileStatus, Hunk, LineMark, RepoDiff,
    Row, RowKind, TEXT_MAX_BYTES, subject_of,
};
pub use editing::{Caret, Edit, Indent, Motion, Selection};
pub use environment::{Found, Tool};
pub use error::{Error, ErrorKind, Result};
pub use ids::{AnnotationId, ApprovalId, ExternalId, MrId, RepoId, SessionId, WorktreeId};
pub use mapping::{Kind, Mapped, Mapping, Property};
pub use mr::{
    CiState, CiStatus, Forge, Mr, MrApproval, MrDetails, MrNote, MrState, MrThread, NotePosition,
    ReviewState, ReviewVerdict, Reviewer,
};
pub use naming::{explorer_branch, is_explorer_branch, names_session};
pub use narrowing::{narrows, occurrences, score};
pub use note::{Anchor, Note, NoteOrigin, Said};
pub use panes::Panes;
pub use repo::{PoolEntry, Repo, Worktree, WorktreeSpec, WorktreeStatus};
pub use review::{ReviewMr, review_for, review_of};
pub use routine::{Action, Routine, RoutineKind, Trigger};
pub use secret::Secret;
pub use session::{Session, SessionKind, SessionState, Skill};
pub use syntax::{Capture, Highlight};
pub use task::{Priority, ProviderId, StatusIntent, Task, TaskDates, TaskKey, TimeSummary, hours};
pub use terminal::{AnsiPalette, Rgb, Screen, ScreenCell, Selected};
pub use time::{Day, Timestamp};
pub use timeline::{TimelineEvent, TimelineKind};
