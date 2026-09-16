//! Every number the ui draws with. Nothing else in the crate holds a size.

/// Unitless and non-pixel values, which no scale touches.
pub const PALETTE_ROWS: usize = 8;
pub const SPINNER_MS: u64 = 120;
pub const SCRIM_ALPHA: u8 = 120;
/// A diff row's ground, under its text.
pub const GROUND_ALPHA: u8 = 38;

/// The smallest a dragged column may be, in logical pixels.
pub const RAIL_MIN: f32 = 160.0;
pub const AGENT_MIN: f32 = 280.0;
pub const WORKSPACE_MIN: f32 = 320.0;
pub const SIDEBAR_MIN: f32 = 180.0;

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
    pub agent: f32,
    pub sidebar: f32,
    /// One line of text, with its leading.
    pub line: f32,
    pub hairline: f32,
    /// How wide a splitter is to grab.
    pub grab: f32,
    /// The type scale. Mono is only for code: the agent, the terminal, the diff.
    pub text: f32,
    pub small: f32,
    pub title: f32,
    pub heading: f32,
    pub code: f32,
    /// One icon's box.
    pub icon: f32,
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
    header: 36.0,
    rail: 220.0,
    agent: 380.0,
    sidebar: 260.0,
    line: 18.0,
    hairline: 1.0,
    grab: 8.0,
    text: 13.0,
    small: 11.5,
    title: 14.0,
    heading: 12.0,
    code: 12.5,
    icon: 16.0,
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
            agent: s(LOGICAL.agent),
            sidebar: s(LOGICAL.sidebar),
            line: s(LOGICAL.line),
            hairline: s(LOGICAL.hairline),
            grab: s(LOGICAL.grab),
            text: s(LOGICAL.text),
            small: s(LOGICAL.small),
            title: s(LOGICAL.title),
            heading: s(LOGICAL.heading),
            code: s(LOGICAL.code),
            icon: s(LOGICAL.icon),
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
