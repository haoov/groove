//! What a text means and what it looks like. The only place a colour or a style is built.

use groove_gfx::{Color, Font, Palette, TextStyle, Weight};
use groove_types::{Capture, LineMark, RowKind, ThemeName};

use crate::base::tokens::{GROUND_ALPHA, SCRIM_ALPHA, Tokens, WORD_ALPHA};

/// What a text or a mark means. Colour follows the role, never the other way round.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Text,
    /// Secondary content: a branch, a value.
    Muted,
    /// A label, a heading, a hint.
    Faint,
    /// Quiet to the point of being out of the way.
    Ghost,
    /// Something waits for the user.
    Attention,
    /// Something is happening.
    Working,
    Ok,
    /// Changed, not yet staged.
    Warn,
    Bad,
    /// An MR that landed.
    Merged,
    Accent,
    /// On an accent ground.
    Inverse,
    /// The colour a cluster context was given.
    Hue(groove_types::Hue),
}

/// The styles of one frame: the flavour's colours at the window's scale.
#[derive(Debug, Clone, Copy)]
pub struct Styles {
    palette: Palette,
    tokens: Tokens,
}

impl Styles {
    pub fn new(theme: ThemeName, tokens: Tokens) -> Self {
        Self {
            palette: crate::base::palette::flavour(theme),
            tokens,
        }
    }

    pub fn color(&self, role: Role) -> Color {
        let p = &self.palette;
        match role {
            Role::Text => p.text,
            Role::Muted => p.subtext1,
            Role::Faint => p.subtext0,
            Role::Ghost => p.overlay0,
            Role::Attention => p.peach,
            Role::Working => p.blue,
            Role::Ok => p.green,
            Role::Warn => p.yellow,
            Role::Bad => p.red,
            Role::Merged => p.mauve,
            Role::Accent => p.rosewater,
            Role::Inverse => p.crust,
            Role::Hue(hue) => crate::base::palette::hue(p, hue),
        }
    }

    /// A surface's name.
    pub fn title(&self, role: Role) -> TextStyle {
        self.sans(self.tokens.title, Weight::Medium, role)
    }

    /// A section's name inside a surface; drawn upper case.
    pub fn heading(&self, role: Role) -> TextStyle {
        self.sans(self.tokens.heading, Weight::Medium, role)
    }

    pub fn body(&self, role: Role) -> TextStyle {
        self.sans(self.tokens.text, Weight::Regular, role)
    }

    /// Prose that names something: a tab, a row, a button.
    pub fn label(&self, role: Role) -> TextStyle {
        self.sans(self.tokens.text, Weight::Medium, role)
    }

    pub fn small(&self, role: Role) -> TextStyle {
        self.sans(self.tokens.small, Weight::Regular, role)
    }

    /// A small word that names what was done.
    pub fn strong(&self, role: Role) -> TextStyle {
        self.sans(self.tokens.small, Weight::Bold, role)
    }

    /// Markdown prose: a paragraph at level 0, a heading at 1 to 3.
    pub fn prose(&self, level: u8, role: Role) -> TextStyle {
        match level {
            0 => self.body(role),
            1 => self.sans(self.tokens.h1, Weight::SemiBold, role),
            2 => self.sans(self.tokens.h2, Weight::SemiBold, role),
            _ => self.sans(self.tokens.h3, Weight::SemiBold, role),
        }
    }

    /// Code in prose, as large as the text around it.
    pub fn prose_code(&self, role: Role) -> TextStyle {
        TextStyle {
            font: Font::Mono,
            weight: Weight::Regular,
            size: self.tokens.text,
            color: self.color(role),
        }
    }

    /// The ground under code in prose, inline or a block.
    pub fn prose_code_ground(&self) -> Color {
        self.palette.surface0
    }

    /// `base` with a span's emphasis.
    pub fn emphasised(&self, base: TextStyle, strong: bool, em: bool) -> TextStyle {
        TextStyle {
            font: if em { Font::Italic } else { base.font },
            weight: if strong { Weight::Bold } else { base.weight },
            ..base
        }
    }

    /// Code: the agent, the terminal, the diff and the editor, and nowhere else.
    pub fn code(&self, role: Role) -> TextStyle {
        TextStyle {
            font: Font::Mono,
            weight: Weight::Regular,
            size: self.tokens.code,
            color: self.color(role),
        }
    }

    /// What a grammar's capture looks like.
    pub fn syntax(&self, capture: Capture) -> Color {
        let p = &self.palette;
        match capture {
            Capture::Keyword => p.mauve,
            Capture::Function => p.blue,
            Capture::Type => p.yellow,
            Capture::String => p.green,
            Capture::Number | Capture::Constant => p.peach,
            Capture::Comment => p.overlay1,
            Capture::Attribute => p.teal,
            Capture::Title => p.lavender,
            Capture::Literal => p.green,
            Capture::Link => p.sapphire,
            Capture::Error => p.red,
            Capture::Warning => p.yellow,
        }
    }

    /// The band under what a caret holds: the accent a quarter into the ground.
    pub fn held(&self) -> Color {
        self.palette
            .base
            .mix(self.palette.rosewater, crate::base::tokens::HELD)
    }

    /// A role's colour dimmed to a ground, as a badge stands on.
    pub fn tint(&self, role: Role) -> Color {
        self.color(role)
            .with_alpha(crate::base::tokens::BADGE_GROUND)
    }

    /// The rules above and below a list's selected row.
    pub fn chosen(&self) -> Color {
        self.palette.rosewater
    }

    /// The rules above and below the caret's row, as a chosen list row has.
    pub fn here(&self) -> Color {
        self.chosen()
    }

    /// The bar where the caret sits.
    pub fn caret(&self) -> Color {
        self.palette.text
    }

    /// What a marked line did: came, went, or changed in place.
    pub fn mark(&self, mark: LineMark) -> Color {
        match mark {
            LineMark::Added => self.palette.green,
            LineMark::Removed => self.palette.red,
            LineMark::Changed => self.palette.yellow,
        }
    }

    /// The ground a diff row stands on, and the one under the caret.
    pub fn row_ground(&self, kind: RowKind) -> Option<Color> {
        let tint = |color: Color| Some(color.with_alpha(GROUND_ALPHA));
        match kind {
            RowKind::Added => tint(self.palette.green),
            RowKind::Removed => tint(self.palette.red),
            _ => None,
        }
    }

    /// Over the ground, where a row and the one it pairs with differ.
    pub fn word(&self, kind: RowKind, mark: Option<LineMark>) -> Option<Color> {
        let tint = |color: Color| Some(color.with_alpha(WORD_ALPHA));
        match kind {
            RowKind::Added => tint(self.palette.green),
            RowKind::Removed => tint(self.palette.red),
            _ => mark.map(|mark| self.mark(mark).with_alpha(WORD_ALPHA)),
        }
    }

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

    /// Under a match a search found, as a selection is.
    pub fn found(&self) -> Color {
        self.palette
            .base
            .mix(self.palette.rosewater, crate::base::tokens::FOUND)
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

    fn sans(&self, size: f32, weight: Weight, role: Role) -> TextStyle {
        TextStyle {
            font: Font::Sans,
            weight,
            size,
            color: self.color(role),
        }
    }
}
