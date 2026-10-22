use groove_gfx::{Color, Rect};

use crate::base::ctx::{App, Ctx};
use crate::base::ground::Ground;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Material {
    Solid,
    Glass,
}

/// A panel with its named corner at `at`, edged in `border`, inside the window.
pub fn panel_at<A: App>(
    ctx: &mut Ctx<'_, A>,
    at: (f32, f32),
    corner: Corner,
    size: (f32, f32),
    (border, panel): (Color, Material),
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
    made(ctx, rect, border, panel);
    rect
}

/// A box centred on the window, edged in `border`, on its own layer. Returns the box.
pub fn modal<A: App>(
    ctx: &mut Ctx<'_, A>,
    (width, height): (f32, f32),
    top: f32,
    (border, panel): (Color, Material),
) -> Rect {
    let window = ctx.window;
    ctx.layer();
    match panel {
        Material::Solid => ctx.quad(window, ctx.styles.scrim()),
        Material::Glass => ctx.veil(ctx.styles.glass_dim()),
    }
    let w = width.min(window.w - ctx.tokens.xl);
    let rect = Rect::new((window.w - w) / 2.0, top, w, height);
    made(ctx, rect, border, panel);
    rect
}

fn made<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect, border: Color, panel: Material) {
    match panel {
        Material::Solid => {
            lifted(ctx, rect);
            let band = ctx.styles.band();
            ctx.rounded_box(rect, band, border, ctx.tokens.round);
        }
        Material::Glass => glass_pane(ctx, rect),
    }
}

/// A rounded pane of glass over `rect`.
pub fn glass_pane<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect) {
    let shape = (ctx.tokens.glass_round, ctx.tokens.bezel);
    ctx.glass(rect, shape, ctx.styles.glass());
}

/// A pane of glass around `content`, `pad` wider on each side and as tall as a bar. Returns the pane.
pub fn glass_pill<A: App>(ctx: &mut Ctx<'_, A>, content: Rect, pad: f32) -> Rect {
    let tall = ctx.tokens.bar;
    let top = content.y + (content.h - tall) / 2.0;
    let pill = Rect::new(content.x - pad, top, content.w + pad * 2.0, tall);
    glass_pane(ctx, pill);
    pill
}

/// `rect` on `ground`, which glass drawn over it shows blurred.
pub fn ground_under<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect, ground: Ground) {
    ctx.under(rect, ground.color(&ctx.styles));
}

/// `place` on `ground`, a pane of glass floating inside it. Returns the pane.
pub fn floating_pane<A: App>(ctx: &mut Ctx<'_, A>, place: Rect, ground: Ground) -> Rect {
    ground_under(ctx, place, ground);
    let pane = place.pad(groove_gfx::Edges::all(ctx.tokens.sm));
    glass_pane(ctx, pane);
    pane
}

/// A soft shade under the box, dropped below it.
fn lifted<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect) {
    let blur = ctx.tokens.md;
    let shade = Rect::new(rect.x, rect.y + ctx.tokens.sm, rect.w, rect.h);
    let color = ctx.styles.shadow();
    ctx.shadow(shade, color, blur);
}

/// Which corner of the panel sits at the point it was opened from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Corner {
    TopLeft,
    BottomLeft,
    BottomRight,
}
