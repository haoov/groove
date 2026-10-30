//! Every number the ui draws with. Nothing else in the crate holds a size.

/// Unitless and non-pixel values, which no scale touches.
pub const PALETTE_ROWS: usize = 8;
/// Lines the centred palette shows, its headings counted.
pub const PALETTE_LINES: usize = 14;
/// How many suggestions the board's filter shows at once.
pub const SUGGESTED_ROWS: usize = 6;
/// The characters a note row holds before the frame has measured, and the fewest.
pub const NOTE_COLS: usize = 80;
pub const NOTE_COLS_FEWEST: usize = 20;
pub const SPINNER_MS: u64 = 120;
/// How far a text may overrun its width and still stand whole.
pub const ELIDE_SLACK: f32 = 0.5;
pub const SCRIM_ALPHA: u8 = 120;
pub const GLASS_ALPHA: (u8, u8) = (140, 95);
pub const GLASS_DIM_ALPHA: (u8, u8) = (143, 59);
pub const GLASS_CODE_ALPHA: (u8, u8) = (110, 140);
pub const ROW_TINT_ALPHA: (u8, u8) = (28, 64);
pub const GLASS_HOVER_ALPHA: (u8, u8) = (18, 22);
pub const GLASS_CARD_ALPHA: (u8, u8) = (12, 16);
pub const GLASS_CHOSEN_ALPHA: (u8, u8) = (38, 46);
pub const GLASS_ACTIVE_ALPHA: (u8, u8) = (51, 64);
/// A diff row's ground, under its text: on a dark theme, then on a light one.
pub const GROUND_ALPHA: (u8, u8) = (12, 28);
pub const NUMBER_TINT: f32 = 0.4;
/// The shade under what stands over the rows.
pub const SHADOW_ALPHA: (u8, u8) = (110, 36);
/// The stripes of a side with no line.
pub const HATCH_ALPHA: (u8, u8) = (90, 130);

/// How much of the note's own colour a line carrying one takes.
pub const NOTED: f32 = 0.18;
/// The words of a row its pair does not have, over that ground.
pub const WORD_ALPHA: (u8, u8) = (44, 80);
/// A badge's ground and edge, both of its word's own colour.
pub const BADGE_GROUND: u8 = 25;
pub const BADGE_EDGE: u8 = 90;

/// A line of prose against its type size.
pub const LEADING: f32 = 1.5;

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

/// How long the caret rests before its blame shows, and the pointer before its popup, in ms.
pub const REST_MS: u64 = 250;
/// How long the caret rests on a word before it and its occurrences are grounded, in ms.
pub const WORD_MS: u64 = 125;
/// The most lines a popup holds.
pub const POPUP_LINES: usize = 14;
/// The most rows the completion list shows at once.
pub const COMPLETION_ROWS: usize = 10;

/// The smallest a dragged column may be, in logical pixels.
pub const RAIL_MIN: f32 = 160.0;
/// How many lines one notch of a wheel carries the agent's screen.
pub const NOTCH: f32 = 3.0;

/// The feed and the footer under it, at their shortest.
pub const FEED_MIN: f32 = 96.0;

/// What the rail's own rows keep whatever the feed takes.
pub const SESSIONS_MIN: f32 = 120.0;

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
/// How much of the conversation's width one side's rows take.
pub const SPEAKER_SHARE: f32 = 0.9;

/// How much of the raised ground stands under a selection and under the word at the caret.
pub const HELD: f32 = 0.8;
pub const OCCURS: f32 = 0.4;
/// How much of the comment colour a scope's guide takes.
pub const GUIDE: f32 = 0.55;
/// How much of the accent stands under a search match.
pub const FOUND: f32 = 0.25;

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
    /// One row of the code surface.
    pub code_line: f32,
    /// One line of the feed, which holds two texts, and its own age column.
    pub feed_row: f32,
    pub feed_age: f32,
    /// The agent's own row, under its screen.
    pub bar: f32,
    pub hairline: f32,
    /// How far a small rounded box's corners give way.
    pub corner: f32,
    /// How far a button's, a menu's and a picker's corners give way.
    pub round: f32,
    /// A glass panel's corners, the width of its curved rim, and one line of a list on it.
    pub glass_round: f32,
    pub bezel: f32,
    pub glass_row: f32,
    pub edge: f32,
    /// How wide a splitter is to grab.
    pub grab: f32,
    /// The column that holds the whole change.
    pub map: f32,
    /// The gap between a hatch's stripes.
    pub hatch: f32,
    /// The type scale. Mono is only for code: the agent, the terminal, the diff.
    pub text: f32,
    pub small: f32,
    pub title: f32,
    pub heading: f32,
    /// A Markdown heading's size, by level.
    pub h1: f32,
    pub h2: f32,
    pub h3: f32,
    pub code: f32,
    /// One icon's box.
    pub icon: f32,
    /// Where a row's second text sits, from the row's left edge.
    pub aside_near: f32,
    pub aside_mid: f32,
    pub aside_far: f32,
    /// The palette's box; `palette` is also the agent's prompt at its widest.
    pub modal: f32,
    pub palette: f32,
    pub modal_top: f32,
    /// What the right button opens.
    pub menu: f32,
    /// What the language servers say of a spot, under it.
    pub popup: f32,
}

/// The type scale and the bands that hold it, against the size they follow.
pub const SMALL: f32 = LOGICAL.small / LOGICAL.text;
const TITLE: f32 = LOGICAL.title / LOGICAL.text;
const HEADING: f32 = LOGICAL.heading / LOGICAL.text;
const H1: f32 = LOGICAL.h1 / LOGICAL.text;
const H2: f32 = LOGICAL.h2 / LOGICAL.text;
const H3: f32 = LOGICAL.h3 / LOGICAL.text;
const ICON: f32 = LOGICAL.icon / LOGICAL.text;
const ROW: f32 = LOGICAL.row / LOGICAL.text;
const GLASS_ROW: f32 = LOGICAL.glass_row / LOGICAL.text;
const HEADER: f32 = LOGICAL.header / LOGICAL.text;
const LINE: f32 = LOGICAL.line / LOGICAL.code;
const CODE_LINE: f32 = LOGICAL.code_line / LOGICAL.code;
const FEED_ROW: f32 = LOGICAL.feed_row / LOGICAL.text;
const BAR: f32 = LOGICAL.bar / LOGICAL.text;
const FEED_AGE: f32 = LOGICAL.feed_age / LOGICAL.text;

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
    code_line: 21.0,
    feed_row: 50.0,
    feed_age: 30.0,
    bar: 36.0,
    hairline: 1.0,
    corner: 4.0,
    round: 2.0,
    glass_round: 18.0,
    bezel: 24.0,
    glass_row: 32.0,
    edge: 0.5,
    grab: 8.0,
    map: 14.0,
    hatch: 6.0,
    text: 13.0,
    small: 11.5,
    title: 16.0,
    heading: 12.0,
    h1: 20.0,
    h2: 17.0,
    h3: 14.5,
    code: 12.5,
    icon: 16.0,
    aside_near: 90.0,
    aside_mid: 160.0,
    aside_far: 320.0,
    modal: 560.0,
    palette: 720.0,
    modal_top: 96.0,
    menu: 150.0,
    popup: 560.0,
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
            line: row(LINE, code),
            code_line: row(CODE_LINE, code),
            feed_row: row(FEED_ROW, text),
            bar: row(BAR, text),
            feed_age: of_text(FEED_AGE),
            hairline: s(LOGICAL.hairline),
            corner: s(LOGICAL.corner),
            round: s(LOGICAL.round),
            glass_round: s(LOGICAL.glass_round),
            bezel: s(LOGICAL.bezel),
            glass_row: row(GLASS_ROW, text),
            edge: s(LOGICAL.edge),
            grab: s(LOGICAL.grab),
            map: s(LOGICAL.map),
            hatch: s(LOGICAL.hatch),
            text: s(text),
            small: of_text(SMALL),
            title: of_text(TITLE),
            heading: of_text(HEADING),
            h1: of_text(H1),
            h2: of_text(H2),
            h3: of_text(H3),
            code: s(code),
            icon: of_text(ICON),
            aside_near: s(LOGICAL.aside_near),
            aside_mid: s(LOGICAL.aside_mid),
            aside_far: s(LOGICAL.aside_far),
            modal: s(LOGICAL.modal),
            palette: s(LOGICAL.palette),
            modal_top: s(LOGICAL.modal_top),
            menu: s(LOGICAL.menu),
            popup: s(LOGICAL.popup),
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
