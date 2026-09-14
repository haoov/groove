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
        }
    }
}
