//! What a mark means. The shape it becomes is the renderer's.

use groove_gfx::Icon;
use groove_types::{ProviderId, SessionKind};

/// An icon by meaning. A view never names a shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    Task,
    Explorer,
    Review,
    Board,
    Settings,
    /// Something is in flight; drawn turning.
    Busy,
    Repo,
    /// Commits this branch has and origin does not.
    Ahead,
    Behind,
    Staged,
    Modified,
    /// A file the reader has marked read.
    Read,
    /// What a search bar carries.
    Search,
    /// What opens a picker.
    Down,
    /// What folds the sidebar away.
    Sidebar,
    /// A check that failed.
    Failed,
    /// A note somebody left on the MR.
    Note,
    /// What takes a thing away.
    Close,
    /// Where a task comes from.
    Github,
    Notion,
    /// What opens a thing where it lives, outside Groove.
    Outward,
    /// A task list's item, open and done.
    Unticked,
    Ticked,
    /// A directory of a file list, shut and open.
    Folder,
    FolderOpen,
}

impl Mark {
    /// The turn that points a mark the other way.
    pub const UPWARDS: u8 = Icon::TURNS / 2;
    /// The turn that points it to the right.
    pub const RIGHTWARDS: u8 = Icon::TURNS * 3 / 4;

    /// The mark a session wears for its kind.
    pub fn of_kind(kind: &SessionKind) -> Self {
        match kind {
            SessionKind::Task { .. } => Mark::Task,
            SessionKind::Explorer => Mark::Explorer,
            SessionKind::Review { .. } => Mark::Review,
        }
    }

    /// The mark of the source a task is read from.
    pub fn of_source(source: ProviderId) -> Self {
        match source {
            ProviderId::Github => Mark::Github,
            ProviderId::Notion => Mark::Notion,
        }
    }

    pub fn shape(self) -> Icon {
        match self {
            Mark::Task => Icon::Flag,
            Mark::Explorer => Icon::Compass,
            Mark::Review => Icon::PullRequest,
            Mark::Board => Icon::Kanban,
            Mark::Settings => Icon::Gear,
            Mark::Busy => Icon::Notch,
            Mark::Repo => Icon::Cube,
            Mark::Ahead => Icon::ArrowUp,
            Mark::Behind => Icon::ArrowDown,
            Mark::Staged => Icon::Plus,
            Mark::Modified => Icon::Dot,
            Mark::Read => Icon::Check,
            Mark::Search => Icon::Glass,
            Mark::Down => Icon::CaretDown,
            Mark::Sidebar => Icon::Sidebar,
            Mark::Failed => Icon::Cross,
            Mark::Note => Icon::Chat,
            Mark::Close => Icon::Cross,
            Mark::Github => Icon::Github,
            Mark::Notion => Icon::Notion,
            Mark::Outward => Icon::Outward,
            Mark::Unticked => Icon::Box,
            Mark::Ticked => Icon::Ticked,
            Mark::Folder => Icon::Folder,
            Mark::FolderOpen => Icon::FolderOpen,
        }
    }
}
