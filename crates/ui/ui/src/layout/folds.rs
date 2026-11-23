//! What the user folds, and what an open panel takes from a folded neighbour.

use groove_gfx::Rect;

use super::Layout;
use crate::Ui;
use groove_ui_kit::base::ctx::Metrics;
use groove_ui_kit::base::tokens::Tokens;

/// Every panel the user folds or opens, as the layout reads them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Folds {
    pub feed: bool,
    pub sessions: bool,
    pub strip: bool,
    pub manual: bool,
    pub sidebar: bool,
    pub alerts: bool,
}

impl Folds {
    pub fn of(ui: &Ui) -> Self {
        let asked = ui.examining().is_some() || ui.losing() == Some(&crate::Losing::Everything);
        Self {
            feed: ui.rail.folded,
            sessions: ui.rail.stowed,
            strip: ui.session.stowed && !asked,
            manual: ui.session.manual,
            sidebar: ui.session.sidebar(),
            alerts: ui.alerts.open,
        }
    }
}

impl Layout {
    /// Where everything stands with the folds the user left.
    pub fn of(metrics: Metrics, ui: &Ui) -> Self {
        Self::folded(metrics, ui, Folds::of(ui))
    }

    /// A rail window folds to its header; the other, open, takes the whole column but that header.
    pub(super) fn rail_folds(mut self, tokens: &Tokens, folds: Folds) -> Self {
        if folds.feed {
            let feed = self.feed;
            let tall = tokens.bar.min(feed.h);
            self.feed = Rect::new(feed.x, feed.bottom() - tall, feed.w, tall);
        }
        if folds.sessions {
            self.sessions.h = tokens.bar.min(self.sessions.h);
        }
        match (folds.sessions, folds.feed) {
            (false, true) => self.sessions.h = (self.feed.y - tokens.sm - self.sessions.y).max(0.0),
            (true, false) => {
                let (feed, top) = (self.feed, self.sessions.bottom() + tokens.sm);
                self.feed = Rect::new(feed.x, top, feed.w, (feed.bottom() - top).max(0.0));
            }
            _ => {}
        }
        self
    }
}
