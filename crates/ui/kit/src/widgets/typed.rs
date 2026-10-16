//! The marks typed text carries: the caret's bar and the band under a span of it.

use groove_gfx::{Color, Rect};

use crate::base::ctx::{App, Ctx};

#[cfg(test)]
mod tests;

/// The caret: a bar at `x`, as tall as `span` and kept within it.
pub fn caret_bar<A: App>(ctx: &mut Ctx<'_, A>, x: f32, span: Rect, color: Color) {
    let width = ctx.tokens.hairline * 2.0;
    let x = x.min(span.right() - width).max(span.x);
    ctx.quad(Rect::new(x, span.y, width, span.h), color);
}

/// A wavy line at the foot of `rect`, from `start` to `end` across it.
pub fn wavy<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect, (start, end): (f32, f32), color: Color) {
    let step = ctx.tokens.hairline;
    let foot = rect.bottom() - step * 2.0;
    let (mut x, end) = (rect.x + start, rect.x + end.max(start + ctx.advance));
    for rise in [0, 1, 2, 1].into_iter().cycle() {
        if x >= end {
            break;
        }
        let y = foot - rise as f32 * step;
        ctx.quad(Rect::new(x, y, step.min(end - x), step), color);
        x += step;
    }
}

/// A band of `color` under `rect`, from `start` to `end` across it.
pub fn text_band<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect, (start, end): (f32, f32), color: Color) {
    let band = Rect::new(rect.x + start, rect.y, (end - start).max(1.0), rect.h);
    ctx.quad(band, color);
}
