use groove_gfx::Rect;

use crate::Corner;
use crate::ctx::Ctx;

/// A panel of its own at `at`, the named corner of it, kept inside the window and
/// dimming nothing behind it.
pub fn panel_at(ctx: &mut Ctx, at: (f32, f32), corner: Corner, size: (f32, f32)) -> Rect {
    let window = ctx.layout.window;
    let (width, height) = size;
    let (x, y) = match corner {
        Corner::TopLeft => at,
        Corner::BottomRight => (at.0 - width, at.1 - height),
    };
    let x = x.min(window.right() - width).max(window.x);
    let y = y.min(window.bottom() - height).max(window.y);
    let rect = Rect::new(x, y, width, height);
    ctx.layer();
    ctx.quad(rect, ctx.styles.panel());
    ctx.border(rect, ctx.styles.border());
    rect
}

/// A box centred on a dimmed window, on its own layer. Returns the box.
pub fn modal(ctx: &mut Ctx, width: f32, height: f32, top: f32) -> Rect {
    let window = ctx.layout.window;
    let (scrim, panel, border) = (ctx.styles.scrim(), ctx.styles.panel(), ctx.styles.border());
    ctx.layer();
    ctx.quad(window, scrim);
    let w = width.min(window.w - ctx.tokens.xl);
    let rect = Rect::new((window.w - w) / 2.0, top, w, height);
    ctx.quad(rect, panel);
    ctx.border(rect, border);
    rect
}
