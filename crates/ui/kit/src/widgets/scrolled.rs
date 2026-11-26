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

/// What `draw` lays from the top of `body`, `offset` scrolled away; returns how far it scrolls.
pub fn flowed<'c, A: App>(
    ctx: &mut Ctx<'c, A>,
    body: Rect,
    offset: f32,
    draw: impl FnOnce(&mut Ctx<'c, A>, &mut Rect),
) -> f32 {
    let start = body.y - offset;
    let mut room = Rect {
        y: start,
        h: f32::INFINITY,
        ..body
    };
    ctx.clipped(body, |ctx| draw(ctx, &mut room));
    (room.y - start - body.h).max(0.0)
}

/// A thumb for `offset` in `extent` down `body`'s right edge; returns its track, if it scrolls.
pub fn scrollbar<A: App>(
    ctx: &mut Ctx<'_, A>,
    body: Rect,
    (offset, extent): (f32, f32),
) -> Option<Rect> {
    if extent <= 0.0 || body.h <= 0.0 {
        return None;
    }
    let tall = thumb_height(body.h, extent, ctx.tokens.row);
    let down = (offset / extent).clamp(0.0, 1.0) * (body.h - tall).max(0.0);
    let wide = ctx.tokens.xs;
    let thumb = Rect::new(body.right() - wide, body.y + down, wide, tall);
    ctx.quad(thumb, ctx.styles.panel_edge());
    let track = ctx.tokens.sm;
    Some(Rect::new(body.right() - track, body.y, track, body.h))
}

/// How tall a thumb stands on a track `high`, for a list that scrolls `extent` past it.
pub fn thumb_height(high: f32, extent: f32, least: f32) -> f32 {
    (high * high / (high + extent)).max(least)
}
