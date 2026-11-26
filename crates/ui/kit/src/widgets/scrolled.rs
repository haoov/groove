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

/// A thumb down the right edge of `body` for `offset` in `extent`; none while every row shows.
pub fn scrollbar<A: App>(ctx: &mut Ctx<'_, A>, body: Rect, (offset, extent): (f32, f32)) {
    if extent <= 0.0 || body.h <= 0.0 {
        return;
    }
    let tall = (body.h * body.h / (body.h + extent)).max(ctx.tokens.row);
    let down = (offset / extent).clamp(0.0, 1.0) * (body.h - tall).max(0.0);
    let wide = ctx.tokens.xs;
    let thumb = Rect::new(body.right() - wide, body.y + down, wide, tall);
    ctx.quad(thumb, ctx.styles.panel_edge());
}
