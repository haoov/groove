//! Rows stacked from the top, scrolled and clipped to their room.

use groove_gfx::Rect;

use crate::ctx::Ctx;
use crate::hit::Scroller;

pub fn scrolled<T>(
    ctx: &mut Ctx,
    body: Rect,
    (which, offset): (Scroller, f32),
    items: &[T],
    height: impl Fn(&T) -> f32,
    mut draw: impl FnMut(&mut Ctx, Rect, &T),
) {
    let content: f32 = items.iter().map(&height).sum();
    let extent = (content - body.h).max(0.0);
    ctx.scrolls(which, extent);
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
}
