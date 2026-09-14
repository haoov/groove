//! What a text means and what it looks like. The only place a colour or a style is built.

use groove_gfx::{Color, Font, Palette, TextStyle, Weight};
use groove_types::ThemeName;

use crate::tokens::{SCRIM_ALPHA, Tokens};

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
    Accent,
    /// On an accent ground.
    Inverse,
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
            palette: flavour(theme),
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
            Role::Accent => p.lavender,
            Role::Inverse => p.crust,
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

    /// The window's own ground.
    pub fn ground(&self) -> Color {
        self.palette.base
    }

    /// A band beside the ground: the rail, the header.
    pub fn panel(&self) -> Color {
        self.palette.mantle
    }

    /// A selected row or tab.
    pub fn raised(&self) -> Color {
        self.palette.surface0
    }

    pub fn line(&self) -> Color {
        self.palette.surface0
    }

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

/// The flavour the config names.
fn flavour(theme: ThemeName) -> Palette {
    match theme {
        ThemeName::Latte => Palette::LATTE,
        ThemeName::Frappe => Palette::FRAPPE,
        ThemeName::Macchiato => Palette::MACCHIATO,
        ThemeName::Mocha => Palette::MOCHA,
    }
}
