//! What a text means and what it looks like. The only place a colour or a style is built.

use groove_gfx::{Color, Font, Palette, TextStyle, Weight};
use groove_types::{Capture, ThemeName};

mod diff;
mod editor;
mod grounds;

use crate::base::tokens::{HATCH_ALPHA, SHADOW_ALPHA, Tokens};

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

    /// A file's name where it heads its own rows.
    pub fn file_head(&self, role: Role) -> TextStyle {
        self.sans(self.tokens.h3, Weight::Medium, role)
    }

    pub fn body(&self, role: Role) -> TextStyle {
        self.sans(self.tokens.text, Weight::Regular, role)
    }

    /// Prose that names something: a tab, a row, a button.
    pub fn label(&self, role: Role) -> TextStyle {
        self.sans(self.tokens.text, Weight::Medium, role)
    }

    /// A name cut to its initials.
    pub fn initials(&self, role: Role) -> TextStyle {
        self.sans(self.tokens.small, Weight::Bold, role)
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

    /// The editor's own text: light where the mono family has a light face, else regular.
    pub fn source(&self, role: Role) -> TextStyle {
        TextStyle {
            weight: Weight::Light,
            ..self.code(role)
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

    /// The alpha a tint takes on this theme, stronger on a light ground.
    fn alpha(&self, (dark, light): (u8, u8)) -> u8 {
        let base = self.palette.base;
        let lightness = u16::from(base.r) + u16::from(base.g) + u16::from(base.b);
        if lightness > 384 { light } else { dark }
    }

    /// The stripes of a side with no line.
    pub fn hatch(&self) -> Color {
        self.palette.surface1.with_alpha(self.alpha(HATCH_ALPHA))
    }

    /// The shade under what stands over the rows.
    pub fn shadow(&self) -> Color {
        Color::BLACK.with_alpha(self.alpha(SHADOW_ALPHA))
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
