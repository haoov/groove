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
            Scroller::Across => self.session.across(),
            Scroller::Overview => self.session.overview,
            Scroller::Aside => self.aside,
            Scroller::Settings => self.settings.scroll,
            Scroller::Resources => self.session.resources.scroll(),
            Scroller::Kinds => self.session.resources.kinds,
            Scroller::Chat => self.agent.scroll,
            Scroller::Prompt => self.agent.prompt.scroll,
            Scroller::Column(at) => self.board.scroll(List::ALL[at as usize]),
        }
    }

    pub(crate) fn set_offset(&mut self, which: Scroller, to: f32) {
        match which {
            Scroller::Rail => self.rail.scroll = to,
            Scroller::Feed => self.rail.feed = to,
            Scroller::Files => self.session.files = to,
            Scroller::Code => *self.session.scroll_mut() = to,
            Scroller::Across => *self.session.across_mut() = to,
            Scroller::Overview => self.session.overview = to,
            Scroller::Aside => self.aside = to,
            Scroller::Settings => self.settings.scroll = to,
            Scroller::Resources => *self.session.resources.scroll_mut() = to,
            Scroller::Kinds => self.session.resources.kinds = to,
            Scroller::Chat => self.agent.scroll = to,
            Scroller::Prompt => self.agent.prompt.scrolled(to),
            Scroller::Column(at) => self.board.scrolled(List::ALL[at as usize], to),
        }
    }

    /// A scrollbar's thumb centred on `y` along its track, the list following it.
    pub(crate) fn thumbed(&mut self, which: Scroller, y: f32, hits: &Hits, least: f32) {
        let Some(track) = hits.rect_of(&crate::hit::Target::Thumb(which)) else {
            return;
        };
        let extent = hits.extent(which);
        let tall = groove_ui_kit::widgets::thumb_height(track.h, extent, least);
        let room = (track.h - tall).max(1.0);
        let share = ((y - track.y - tall / 2.0) / room).clamp(0.0, 1.0);
        self.set_offset(which, share * extent);
    }

    /// The wheel moves one scroller, inside what it could scroll when last drawn.
    pub(crate) fn wheeled(&mut self, which: Scroller, pixels: f32, hits: &Hits) {
        let to = (self.offset(which) - pixels).clamp(0.0, hits.extent(which));
        self.set_offset(which, to);
    }
}

/// A list the wheel scrolls, at the scroller's offset, its extent kept for the next turn and returned.
pub(crate) fn listed<'c, T>(
    ctx: &mut Ctx<'c>,
    body: Rect,
    (which, offset): (Scroller, f32),
    items: &[T],
    height: impl Fn(&T) -> f32,
    draw: impl FnMut(&mut Ctx<'c>, Rect, &T),
) -> f32 {
    let extent = groove_ui_kit::widgets::scrolled(ctx, body, offset, items, height, draw);
    ctx.app.hits.scrolls(which, extent);
    extent
}
