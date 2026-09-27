//! Where each scroller keeps its offset.

use crate::Ui;
use crate::base::hit::{Hits, Scroller};
use crate::views::board::List;

impl Ui {
    pub(crate) fn offset(&self, which: Scroller) -> f32 {
        match which {
            Scroller::Rail => self.rail.scroll,
            Scroller::Feed => self.rail.feed,
            Scroller::Files => self.session.files,
            Scroller::Code => self.session.diff,
            Scroller::Overview => self.session.overview,
            Scroller::Column(at) => self.board.scroll(List::ALL[at as usize]),
        }
    }

    fn set_offset(&mut self, which: Scroller, to: f32) {
        match which {
            Scroller::Rail => self.rail.scroll = to,
            Scroller::Feed => self.rail.feed = to,
            Scroller::Files => self.session.files = to,
            Scroller::Code => self.session.diff = to,
            Scroller::Overview => self.session.overview = to,
            Scroller::Column(at) => self.board.scrolled(List::ALL[at as usize], to),
        }
    }

    /// The wheel moves one scroller, inside what it could scroll when last drawn.
    pub(crate) fn wheeled(&mut self, which: Scroller, pixels: f32, hits: &Hits) {
        let to = (self.offset(which) - pixels).clamp(0.0, hits.extent(which));
        self.set_offset(which, to);
    }
}
