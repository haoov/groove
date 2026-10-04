//! The grounds a region or a row stands on, by name.

use groove_gfx::Color;
use groove_types::LineMark;

use crate::base::style::{Role, Styles};

/// A ground a view may fill.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Ground {
    /// The work: the workspace, a header, the commit box.
    Work,
    /// A band beside the work or across it: the rail, the sidebar, a heading.
    Band,
    /// A terminal, a note.
    Deep,
    /// A row under the pointer.
    Hover,
    /// A selected row or tab.
    Raised,
    /// What a click acts on.
    Action,
    /// What a caret holds.
    Held,
    /// A line a note stands on.
    Noted,
    /// A role's colour dimmed to a ground.
    Tint(Role),
    /// A role's own colour: a drop point, a working bar.
    Solid(Role),
    /// What a line did: came, went, or changed.
    Mark(LineMark),
}

impl Ground {
    pub fn color(self, styles: &Styles) -> Color {
        match self {
            Self::Work => styles.ground(),
            Self::Band => styles.band(),
            Self::Deep => styles.deep(),
            Self::Hover => styles.hover(),
            Self::Raised => styles.raised(),
            Self::Action => styles.action(),
            Self::Held => styles.held(),
            Self::Noted => styles.noted(),
            Self::Tint(role) => styles.tint(role),
            Self::Solid(role) => styles.color(role),
            Self::Mark(mark) => styles.mark(mark),
        }
    }
}
