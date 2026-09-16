//! What the pointer can reach. A view registers a rect and what it means; a click
//! resolves to the last one registered over that point.

use groove_gfx::Rect;
use groove_types::{SessionId, WorktreeId};

use crate::layout::Edge;
use crate::views::session::Tab;

/// What the pointer looks like over a region.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Cursor {
    #[default]
    Default,
    /// Over something a click acts on.
    Pointer,
    /// Over a boundary a drag moves sideways.
    ColResize,
    /// Over code.
    Text,
}

/// A thing on screen the pointer can act on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// A rail row.
    Session(SessionId),
    /// A tab of the workspace.
    Tab(Tab),
    /// The header's repo or worktree picker.
    Picker,
    /// An overview row.
    Worktree(WorktreeId),
    /// A palette row, by its place in the list.
    PaletteRow(usize),
    /// The palette's box; a click on it does nothing.
    Palette,
    /// A boundary between two columns.
    Split(Edge),
    /// What folds the sidebar away.
    Fold,
    /// A file's row in the sidebar.
    File(String),
    /// The open file's rows.
    Code,
}

impl Target {
    /// What the pointer says over it.
    fn cursor(&self) -> Cursor {
        match self {
            Target::Session(_)
            | Target::Tab(_)
            | Target::Picker
            | Target::Worktree(_)
            | Target::PaletteRow(_)
            | Target::Fold
            | Target::File(_) => Cursor::Pointer,
            Target::Code => Cursor::Text,
            Target::Split(_) => Cursor::ColResize,
            Target::Palette => Cursor::Default,
        }
    }
}

/// Where everything was drawn this frame.
#[derive(Debug, Default)]
pub struct Hits {
    regions: Vec<(Rect, Target)>,
}

impl Hits {
    pub fn push(&mut self, rect: Rect, target: Target) {
        self.regions.push((rect, target));
    }

    /// The last thing drawn over this point.
    pub fn at(&self, x: f32, y: f32) -> Option<Target> {
        self.topmost(x, y).cloned()
    }

    /// The pointer over this point.
    pub fn cursor_at(&self, x: f32, y: f32) -> Cursor {
        self.topmost(x, y).map(Target::cursor).unwrap_or_default()
    }

    fn topmost(&self, x: f32, y: f32) -> Option<&Target> {
        self.regions
            .iter()
            .rev()
            .find(|(rect, _)| rect.contains(x, y))
            .map(|(_, target)| target)
    }

    pub fn is_empty(&self) -> bool {
        self.regions.is_empty()
    }

    /// Where this target was drawn.
    pub fn rect_of(&self, target: &Target) -> Option<Rect> {
        self.regions
            .iter()
            .find(|(_, at)| at == target)
            .map(|(rect, _)| *rect)
    }
}
