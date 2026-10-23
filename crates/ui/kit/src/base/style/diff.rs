//! The colours of a diff: what a line did, and the grounds a changed row takes.

use groove_gfx::Color;
use groove_types::{Capture, LineMark, RowKind, Severity};

use super::Styles;
use crate::base::tokens::{GROUND_ALPHA, NUMBER_TINT, WORD_ALPHA};

impl Styles {
    /// What a marked line did: came, went, or changed in place.
    pub fn mark(&self, mark: LineMark) -> Color {
        match mark {
            LineMark::Added => self.palette.green,
            LineMark::Removed => self.palette.red,
            LineMark::Changed => self.palette.yellow,
        }
    }

    /// The colour of a row that came or went.
    fn changed(&self, kind: RowKind) -> Option<Color> {
        match kind {
            RowKind::Added => Some(self.palette.green),
            RowKind::Removed => Some(self.palette.red),
            _ => None,
        }
    }

    /// The ground a diff row stands on, and the one under the caret.
    pub fn row_ground(&self, kind: RowKind) -> Option<Color> {
        self.changed(kind)
            .map(|color| color.with_alpha(self.alpha(GROUND_ALPHA)))
    }

    /// The gutter of a changed row: its ground over the row's, and its numbers.
    pub fn gutter(&self, kind: RowKind) -> Option<Color> {
        let color = self.changed(kind)?;
        Some(self.palette.overlay0.mix(color, NUMBER_TINT))
    }

    /// Over the ground, where a row and the one it pairs with differ.
    pub fn word(&self, kind: RowKind, mark: Option<LineMark>) -> Option<Color> {
        let color = self
            .changed(kind)
            .or_else(|| mark.map(|mark| self.mark(mark)));
        color.map(|color| color.with_alpha(self.alpha(WORD_ALPHA)))
    }

    /// A syntax colour on a changed row, a comment lifted off the tint.
    pub fn changed_syntax(&self, capture: Capture) -> Color {
        match capture {
            Capture::Comment => self.palette.subtext1,
            _ => self.syntax(capture),
        }
    }

    /// What a language server found, by how bad it is.
    pub fn fault(&self, severity: Severity) -> Color {
        match severity {
            Severity::Error => self.palette.red,
            Severity::Warning => self.palette.yellow,
            Severity::Information => self.palette.blue,
            Severity::Hint => self.palette.overlay0,
        }
    }
}
