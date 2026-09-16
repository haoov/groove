//! What a mark means. The shape it becomes is the renderer's.

use groove_gfx::Icon;
use groove_types::SessionKind;

/// An icon by meaning, so a view never names a shape.
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
    /// What opens a picker.
    Down,
    /// What folds the sidebar away.
    Sidebar,
}

impl Mark {
    /// The mark a session wears for its kind.
    pub fn of_kind(kind: &SessionKind) -> Self {
        match kind {
            SessionKind::Task { .. } => Mark::Task,
            SessionKind::Explorer => Mark::Explorer,
            SessionKind::Review { .. } => Mark::Review,
        }
    }

    pub(crate) fn shape(self) -> Icon {
        match self {
            Mark::Task => Icon::Flag,
            Mark::Explorer => Icon::Compass,
            Mark::Review => Icon::Eye,
            Mark::Board => Icon::Kanban,
            Mark::Settings => Icon::Gear,
            Mark::Busy => Icon::Notch,
            Mark::Repo => Icon::Cube,
            Mark::Ahead => Icon::ArrowUp,
            Mark::Behind => Icon::ArrowDown,
            Mark::Staged => Icon::Plus,
            Mark::Modified => Icon::Dot,
            Mark::Down => Icon::CaretDown,
            Mark::Sidebar => Icon::Sidebar,
        }
    }
}
