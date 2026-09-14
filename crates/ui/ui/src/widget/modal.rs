use groove_gfx::Rect;

use crate::ctx::Ctx;

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
