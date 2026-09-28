//! Where each scroller keeps its offset.

use groove_gfx::Rect;

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::{Hits, Scroller};
use crate::views::board::List;

impl Ui {
    pub(crate) fn offset(&self, which: Scroller) -> f32 {
        match which {
            Scroller::Rail => self.rail.scroll,
            Scroller::Feed => self.rail.feed,
            Scroller::Files => self.session.files,
            Scroller::Code => self.session.scroll(),
            Scroller::Overview => self.session.overview,
            Scroller::Settings => self.settings.scroll,
            Scroller::Column(at) => self.board.scroll(List::ALL[at as usize]),
        }
    }

    fn set_offset(&mut self, which: Scroller, to: f32) {
        match which {
            Scroller::Rail => self.rail.scroll = to,
            Scroller::Feed => self.rail.feed = to,
            Scroller::Files => self.session.files = to,
            Scroller::Code => *self.session.scroll_mut() = to,
            Scroller::Overview => self.session.overview = to,
            Scroller::Settings => self.settings.scroll = to,
            Scroller::Column(at) => self.board.scrolled(List::ALL[at as usize], to),
        }
    }

    /// The wheel moves one scroller, inside what it could scroll when last drawn.
    pub(crate) fn wheeled(&mut self, which: Scroller, pixels: f32, hits: &Hits) {
        let to = (self.offset(which) - pixels).clamp(0.0, hits.extent(which));
        self.set_offset(which, to);
    }
}

/// A list the wheel scrolls, at the scroller's offset, its extent kept for the next turn.
pub(crate) fn listed<'c, T>(
    ctx: &mut Ctx<'c>,
    body: Rect,
    (which, offset): (Scroller, f32),
    items: &[T],
    height: impl Fn(&T) -> f32,
    draw: impl FnMut(&mut Ctx<'c>, Rect, &T),
) {
    let extent = groove_ui_kit::widgets::scrolled(ctx, body, offset, items, height, draw);
    ctx.app.hits.scrolls(which, extent);
}
