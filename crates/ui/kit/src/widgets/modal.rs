use groove_gfx::{Color, Rect};

use crate::base::ctx::{App, Ctx};

/// A panel with its named corner at `at`, edged in `border`, inside the window.
pub fn panel_at<A: App>(
    ctx: &mut Ctx<'_, A>,
    at: (f32, f32),
    corner: Corner,
    size: (f32, f32),
    border: Color,
) -> Rect {
    let window = ctx.window;
    let (width, height) = size;
    let (x, y) = match corner {
        Corner::TopLeft => at,
        Corner::BottomLeft => (at.0, at.1 - height),
        Corner::BottomRight => (at.0 - width, at.1 - height),
    };
    let rect = crate::shape::kept_in((x, y), (width, height), window);
    ctx.layer();
    ctx.quad(rect, ctx.styles.band());
    ctx.border(rect, border);
    rect
}

/// A box centred on a dimmed window, edged in `border`, on its own layer. Returns the box.
pub fn modal<A: App>(
    ctx: &mut Ctx<'_, A>,
    width: f32,
    height: f32,
    top: f32,
    border: Color,
) -> Rect {
    let window = ctx.window;
    let (scrim, panel) = (ctx.styles.scrim(), ctx.styles.band());
    ctx.layer();
    ctx.quad(window, scrim);
    let w = width.min(window.w - ctx.tokens.xl);
    let rect = Rect::new((window.w - w) / 2.0, top, w, height);
    ctx.quad(rect, panel);
    ctx.border(rect, border);
    rect
}

/// Which corner of the panel sits at the point it was opened from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Corner {
    TopLeft,
    BottomLeft,
    BottomRight,
}
