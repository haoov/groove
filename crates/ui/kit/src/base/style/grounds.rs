//! The grounds and rules a row, a band or a panel takes.

use groove_gfx::Color;

use super::Styles;
use crate::base::tokens::SCRIM_ALPHA;

impl Styles {
    /// The work's own ground: the window, the workspace, the agent's pane.
    pub fn ground(&self) -> Color {
        self.palette.base
    }

    /// A band beside the work or across it: the rail, the sidebar, a heading, a menu.
    pub fn band(&self) -> Color {
        self.palette.mantle
    }

    /// The deepest ground, a terminal's own.
    pub fn deep(&self) -> Color {
        self.palette.crust
    }

    /// A row under the pointer, the quietest of the three grounds a row can take.
    pub fn hover(&self) -> Color {
        self.palette.surface0
    }

    /// The ground of something a click acts on, above a raised row.
    pub fn action(&self) -> Color {
        self.palette.surface2
    }

    /// A selected row or tab.
    pub fn raised(&self) -> Color {
        self.palette.surface1
    }

    /// The outline of the map's lens.
    pub fn lens(&self) -> Color {
        self.palette.text
    }

    /// The ground of a line a note stands on, a colour of its own.
    pub fn noted(&self) -> Color {
        self.palette
            .base
            .mix(self.palette.yellow, crate::base::tokens::NOTED)
    }

    /// Under the match the search stands on.
    pub fn standing(&self) -> Color {
        self.palette
            .base
            .mix(self.palette.peach, crate::base::tokens::FOUND)
    }

    pub fn line(&self) -> Color {
        self.palette.surface0
    }

    /// A lighter edge, for a panel that stands over a band.
    pub fn border(&self) -> Color {
        self.palette.surface1
    }

    /// Over the window, under a modal.
    pub fn scrim(&self) -> Color {
        self.palette.crust.with_alpha(SCRIM_ALPHA)
    }
}
