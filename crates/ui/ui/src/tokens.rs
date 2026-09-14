//! Every number the ui draws with. Nothing else in the crate holds a size.

/// Unitless and non-pixel values, which no scale touches.
pub const AGENT_SHARE: f32 = 0.45;
pub const PALETTE_ROWS: usize = 8;
pub const SPINNER_MS: u64 = 120;
pub const SCRIM_ALPHA: u8 = 120;

/// Sizes in pixels, scaled to the window.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tokens {
    pub scale: f32,
    /// The spacing scale.
    pub xs: f32,
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
    pub xl: f32,
    /// One line of a list, and the window's bands.
    pub row: f32,
    pub header: f32,
    pub rail: f32,
    pub hairline: f32,
    /// The type scale.
    pub text: f32,
    pub small: f32,
    pub title: f32,
    pub mono: f32,
    /// A child row under its parent.
    pub indent: f32,
    /// Where a row's second text sits, from the row's left edge.
    pub aside_near: f32,
    pub aside_mid: f32,
    pub aside_far: f32,
    /// The palette's box.
    pub modal: f32,
    pub modal_top: f32,
}

/// The design's numbers, at scale 1.
const LOGICAL: Tokens = Tokens {
    scale: 1.0,
    xs: 4.0,
    sm: 8.0,
    md: 12.0,
    lg: 16.0,
    xl: 24.0,
    row: 26.0,
    header: 32.0,
    rail: 220.0,
    hairline: 1.0,
    text: 12.0,
    small: 11.0,
    title: 14.0,
    mono: 12.5,
    indent: 28.0,
    aside_near: 90.0,
    aside_mid: 160.0,
    aside_far: 320.0,
    modal: 560.0,
    modal_top: 96.0,
};

impl Tokens {
    pub fn new(scale: f32) -> Self {
        let s = |value: f32| value * scale;
        Self {
            scale,
            xs: s(LOGICAL.xs),
            sm: s(LOGICAL.sm),
            md: s(LOGICAL.md),
            lg: s(LOGICAL.lg),
            xl: s(LOGICAL.xl),
            row: s(LOGICAL.row),
            header: s(LOGICAL.header),
            rail: s(LOGICAL.rail),
            hairline: s(LOGICAL.hairline),
            text: s(LOGICAL.text),
            small: s(LOGICAL.small),
            title: s(LOGICAL.title),
            mono: s(LOGICAL.mono),
            indent: s(LOGICAL.indent),
            aside_near: s(LOGICAL.aside_near),
            aside_mid: s(LOGICAL.aside_mid),
            aside_far: s(LOGICAL.aside_far),
            modal: s(LOGICAL.modal),
            modal_top: s(LOGICAL.modal_top),
        }
    }
}

impl Default for Tokens {
    fn default() -> Self {
        LOGICAL
    }
}
