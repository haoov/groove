//! Rows stacked from the top, scrolled and clipped to their room.

use groove_gfx::Rect;

use crate::base::ctx::{App, Ctx};

/// Draws the rows that show at `offset`; returns how far they could scroll.
pub fn scrolled<'c, A: App, T>(
    ctx: &mut Ctx<'c, A>,
    body: Rect,
    offset: f32,
    items: &[T],
    height: impl Fn(&T) -> f32,
    mut draw: impl FnMut(&mut Ctx<'c, A>, Rect, &T),
) -> f32 {
    let content: f32 = items.iter().map(&height).sum();
    let extent = (content - body.h).max(0.0);
    ctx.clipped(body, |ctx| {
        let mut y = body.y - offset.min(extent);
        for item in items {
            let tall = height(item);
            if y + tall >= body.y && y <= body.bottom() {
                draw(ctx, Rect::new(body.x, y, body.w, tall), item);
            }
            y += tall;
        }
    });
    extent
}
