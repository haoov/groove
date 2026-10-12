//! The marks typed text carries: the caret's bar and the band under a span of it.

use groove_gfx::{Color, Rect};

use crate::base::ctx::{App, Ctx};

#[cfg(test)]
mod tests;

/// The caret: a bar at `x`, as tall as `span`.
pub fn caret_bar<A: App>(ctx: &mut Ctx<'_, A>, x: f32, span: Rect, color: Color) {
    let width = ctx.tokens.hairline * 2.0;
    ctx.quad(Rect::new(x, span.y, width, span.h), color);
}

/// A band of `color` under `rect`, from `start` to `end` across it.
pub fn text_band<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect, (start, end): (f32, f32), color: Color) {
    let band = Rect::new(rect.x + start, rect.y, (end - start).max(1.0), rect.h);
    ctx.quad(band, color);
}
