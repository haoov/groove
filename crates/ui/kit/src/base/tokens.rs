//! Every number the ui draws with. Nothing else in the crate holds a size.

/// Unitless and non-pixel values, which no scale touches.
pub const PALETTE_ROWS: usize = 8;
pub const SPINNER_MS: u64 = 120;
pub const SCRIM_ALPHA: u8 = 120;
/// A diff row's ground, under its text.
pub const GROUND_ALPHA: u8 = 38;

/// How much of the note's own colour a line carrying one takes.
pub const NOTED: f32 = 0.18;
/// The words of a row its pair does not have, over that ground.
pub const WORD_ALPHA: u8 = 96;
/// A badge's ground and edge, both of its word's own colour.
pub const BADGE_GROUND: u8 = 25;
pub const BADGE_EDGE: u8 = 90;

/// How many lines of a commit message the box shows.
pub const MESSAGE_LINES: usize = 10;

/// How many scopes stand over the rows, and the share of the surface they may take.
pub const PINNED_DEEP: usize = 3;
pub const PINNED_SHARE: f32 = 4.0;

/// How many rows a search keeps above the match it lands on.
pub const ABOVE_MATCH: usize = 4;

/// How soon and how near a press counts as the same click, in milliseconds and logical pixels.
pub const CLICK_MS: u64 = 400;
pub const CLICK_SLOP: f32 = 4.0;

/// The smallest a dragged column may be, in logical pixels.
pub const RAIL_MIN: f32 = 160.0;
/// How many lines one notch of a wheel carries the agent's screen.
pub const NOTCH: f32 = 3.0;

/// How far a gesture carries the timeline one day.
pub const DAY_PIXELS: f32 = 24.0;

/// The smallest the timeline's band may be, and the room it leaves the columns.
pub const BAND_MIN: f32 = 120.0;

/// The feed and the footer under it, at their shortest.
pub const FEED_MIN: f32 = 96.0;

/// What the rail's own rows keep whatever the feed takes.
pub const SESSIONS_MIN: f32 = 120.0;
pub const COLUMNS_MIN: f32 = 200.0;

/// The smallest the commit box and the list above it may be.
pub const COMMIT_MIN: f32 = 72.0;
pub const FILES_MIN: f32 = 120.0;
/// The manual section open, as tall as it starts and no shorter, and the code it leaves above.
pub const MANUAL_TALL: f32 = 220.0;
pub const MANUAL_MIN: f32 = 96.0;
pub const CODE_MIN: f32 = 160.0;
pub const AGENT_MIN: f32 = 280.0;
pub const WORKSPACE_MIN: f32 = 320.0;
pub const SIDEBAR_MIN: f32 = 180.0;

/// How much of the accent stands in the band under a selection.
pub const HELD: f32 = 0.25;

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
    /// The timeline's band, when it stands open.
    pub band: f32,
    /// One line of text, with its leading.
    pub line: f32,
    /// One line of the feed, which holds two texts, and its own age column.
    pub feed_row: f32,
    pub feed_age: f32,
    /// The agent's own row, under its screen.
    pub bar: f32,
    pub hairline: f32,
    /// How far a small rounded box's corners give way.
    pub corner: f32,
    pub edge: f32,
    /// How wide a splitter is to grab.
    pub grab: f32,
    /// The column that holds the whole change.
    pub map: f32,
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
    /// What the right button opens.
    pub menu: f32,
}

/// The type scale and the bands that hold it, against the size they follow.
const SMALL: f32 = 11.5 / 13.0;
const TITLE: f32 = 16.0 / 13.0;
const HEADING: f32 = 12.0 / 13.0;
const ICON: f32 = 16.0 / 13.0;
const ROW: f32 = 26.0 / 13.0;
const HEADER: f32 = 36.0 / 13.0;
const LINE: f32 = 18.0 / 12.5;
const FEED_ROW: f32 = 34.0 / 13.0;
const BAR: f32 = 36.0 / 13.0;
const FEED_AGE: f32 = 30.0 / 13.0;

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
    band: 260.0,
    line: 18.0,
    feed_row: 34.0,
    feed_age: 30.0,
    bar: 36.0,
    hairline: 1.0,
    corner: 4.0,
    edge: 0.5,
    grab: 8.0,
    map: 14.0,
    text: 13.0,
    small: 11.5,
    title: 16.0,
    heading: 12.0,
    code: 12.5,
    icon: 16.0,
    aside_near: 90.0,
    aside_mid: 160.0,
    aside_far: 320.0,
    modal: 560.0,
    modal_top: 96.0,
    menu: 150.0,
};

impl Tokens {
    /// The design's own sizes, scaled to the window.
    pub fn new(scale: f32) -> Self {
        Self::sized(scale, LOGICAL.text, LOGICAL.code)
    }

    /// The same, with the interface's type at `text` and code's at `code`.
    pub fn sized(scale: f32, text: f32, code: f32) -> Self {
        let s = |value: f32| value * scale;
        let of_text = |ratio: f32| s(text * ratio);
        let row = |ratio: f32, size: f32| s((size * ratio).round());
        Self {
            scale,
            xs: s(LOGICAL.xs),
            sm: s(LOGICAL.sm),
            md: s(LOGICAL.md),
            lg: s(LOGICAL.lg),
            xl: s(LOGICAL.xl),
            row: row(ROW, text),
            header: row(HEADER, text),
            rail: s(LOGICAL.rail),
            agent: s(LOGICAL.agent),
            sidebar: s(LOGICAL.sidebar),
            band: s(LOGICAL.band),
            line: row(LINE, code),
            feed_row: row(FEED_ROW, text),
            bar: row(BAR, text),
            feed_age: of_text(FEED_AGE),
            hairline: s(LOGICAL.hairline),
            corner: s(LOGICAL.corner),
            edge: s(LOGICAL.edge),
            grab: s(LOGICAL.grab),
            map: s(LOGICAL.map),
            text: s(text),
            small: of_text(SMALL),
            title: of_text(TITLE),
            heading: of_text(HEADING),
            code: s(code),
            icon: of_text(ICON),
            aside_near: s(LOGICAL.aside_near),
            aside_mid: s(LOGICAL.aside_mid),
            aside_far: s(LOGICAL.aside_far),
            modal: s(LOGICAL.modal),
            modal_top: s(LOGICAL.modal_top),
            menu: s(LOGICAL.menu),
        }
    }
}

impl Default for Tokens {
    fn default() -> Self {
        LOGICAL
    }
}

/// How much wider a note's characters are taken to be, and how many the author and lines keep.
pub const NOTE_SLACK: f32 = 1.15;
pub const NOTE_BY: f32 = 22.0;
