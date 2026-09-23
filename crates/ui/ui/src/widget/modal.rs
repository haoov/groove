use groove_gfx::{Color, Rect};

use crate::Corner;
use crate::ctx::Ctx;

/// A panel of its own at `at`, the named corner of it, edged in `border`, kept inside
/// the window and dimming nothing behind it.
pub fn panel_at(
    ctx: &mut Ctx,
    at: (f32, f32),
    corner: Corner,
    size: (f32, f32),
    border: Color,
) -> Rect {
    let window = ctx.layout.window;
    let (width, height) = size;
    let (x, y) = match corner {
        Corner::TopLeft => at,
        Corner::BottomLeft => (at.0, at.1 - height),
        Corner::BottomRight => (at.0 - width, at.1 - height),
    };
    let x = x.min(window.right() - width).max(window.x);
    let y = y.min(window.bottom() - height).max(window.y);
    let rect = Rect::new(x, y, width, height);
    ctx.layer();
    ctx.quad(rect, ctx.styles.band());
    ctx.border(rect, border);
    rect
}

/// A box centred on a dimmed window, edged in `border`, on its own layer. Returns the box.
pub fn modal(ctx: &mut Ctx, width: f32, height: f32, top: f32, border: Color) -> Rect {
    let window = ctx.layout.window;
    let (scrim, panel) = (ctx.styles.scrim(), ctx.styles.band());
    ctx.layer();
    ctx.quad(window, scrim);
    let w = width.min(window.w - ctx.tokens.xl);
    let rect = Rect::new((window.w - w) / 2.0, top, w, height);
    ctx.quad(rect, panel);
    ctx.border(rect, border);
    rect
}
