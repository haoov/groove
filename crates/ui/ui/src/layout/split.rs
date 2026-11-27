//! What the user dragged: the rail's and the sidebar's widths, and the heights across.

use groove_types::Panes;

use groove_ui_kit::base::tokens::{
    CODE_MIN, FEED_MIN, MANUAL_MIN, MANUAL_TALL, MIDDLE_MIN, RAIL_MIN, SESSIONS_MIN, SIDEBAR_MIN,
    Tokens,
};

/// A boundary the user drags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    /// Between the rail and the session's middle.
    Rail,
    /// Between the rail's own rows and the feed under them.
    Feed,
    /// Between the session's middle and the terminals under it.
    Manual,
    /// Between the session's middle and its sidebar, as the rail's edge mirrored.
    Aside,
}

impl Edge {
    /// Whether the boundary is a vertical line, which the pointer moves sideways.
    pub fn upright(self) -> bool {
        !matches!(self, Edge::Feed | Edge::Manual)
    }
}

/// The rail's and the sidebar's widths, the middle taking the rest, in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Split {
    pub rail: f32,
    pub sidebar: f32,
    /// How tall the rail's feed stands, the footer under it included.
    pub feed: f32,
    /// How tall the manual section stands open.
    pub manual: f32,
}

impl Default for Split {
    fn default() -> Self {
        let tokens = Tokens::default();
        Self {
            rail: tokens.rail,
            sidebar: tokens.sidebar,
            feed: FEED_MIN * 2.0,
            manual: MANUAL_TALL,
        }
    }
}

impl Split {
    /// What a past run left, no column under its minimum.
    pub fn of(panes: Panes) -> Self {
        Self {
            rail: panes.rail.max(RAIL_MIN),
            sidebar: panes.sidebar.max(SIDEBAR_MIN),
            feed: panes.feed.max(FEED_MIN),
            manual: panes.manual.max(MANUAL_MIN),
        }
    }

    pub fn panes(&self) -> Panes {
        Panes {
            rail: self.rail,
            sidebar: self.sidebar,
            feed: self.feed,
            manual: self.manual,
        }
    }

    /// Puts `edge` at `at`, the middle never under its minimum.
    pub fn drag(&mut self, edge: Edge, at: f32, window: (f32, f32)) {
        let (width, height) = window;
        match edge {
            Edge::Rail => {
                let most = (width - self.sidebar - MIDDLE_MIN).max(RAIL_MIN);
                self.rail = at.clamp(RAIL_MIN, most);
            }
            Edge::Feed => {
                let most = (height - SESSIONS_MIN).max(FEED_MIN);
                self.feed = (height - at).clamp(FEED_MIN, most);
            }
            Edge::Manual => {
                let most = (height - CODE_MIN).max(MANUAL_MIN);
                self.manual = (height - at).clamp(MANUAL_MIN, most);
            }
            Edge::Aside => {
                let most = (width - self.rail - MIDDLE_MIN).max(SIDEBAR_MIN);
                self.sidebar = (width - at).clamp(SIDEBAR_MIN, most);
            }
        }
    }

    pub fn edge_at(&self, edge: Edge, window: (f32, f32)) -> f32 {
        let (width, height) = window;
        match edge {
            Edge::Rail => self.rail,
            Edge::Feed => height - self.feed,
            Edge::Manual => height - self.manual,
            Edge::Aside => width - self.sidebar,
        }
    }
}
