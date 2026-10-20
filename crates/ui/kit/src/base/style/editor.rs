//! The grounds under the editor's text: a selection, the word at the caret, a match.

use groove_gfx::Color;

use super::Styles;

impl Styles {
    /// The band under what a caret holds.
    pub fn held(&self) -> Color {
        self.palette
            .base
            .mix(self.palette.surface1, crate::base::tokens::HELD)
    }

    /// A scope's guide down the indent: the comment colour, part way into the ground.
    pub fn guide(&self) -> Color {
        let comment = self.syntax(groove_types::Capture::Comment);
        self.palette.base.mix(comment, crate::base::tokens::GUIDE)
    }

    /// Under the word the caret rests in and its occurrences, dimmer than a selection.
    pub fn occurs(&self) -> Color {
        self.palette
            .base
            .mix(self.palette.surface1, crate::base::tokens::OCCURS)
    }

    /// Under a match a search found, as a selection is.
    pub fn found(&self) -> Color {
        self.palette
            .base
            .mix(self.palette.rosewater, crate::base::tokens::FOUND)
    }
}
