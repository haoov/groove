use groove_gfx::{Align, Color, Edges, Rect};

use crate::base::ctx::{App, Ctx};
use crate::base::ground::Ground;
use crate::base::mark::Mark;
use crate::base::style::Role;

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

/// An opaque pane standing on the ground beside the others: the theme's base, edged by a hairline.
/// Glass is kept for what floats over something; a panel has nothing under it to show.
pub fn solid_pane<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect) {
    let (ground, edge) = (ctx.styles.ground(), ctx.styles.line());
    ctx.rounded_box(rect, ground, edge, ctx.tokens.glass_round);
}

/// A card set into a panel: a step below its base, edged by a hairline.
pub fn sunk_pane<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect) {
    let (ground, edge) = (ctx.styles.band(), ctx.styles.line());
    ctx.rounded_box(rect, ground, edge, ctx.tokens.glass_round);
}

/// A panel's header band across `rect`, inside the panel's edge: rounded at the top and ruled
/// off under it, or rounded all round when the panel is folded to it alone.
pub fn panel_head<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect, alone: bool) {
    let (edge, round) = (ctx.tokens.hairline, ctx.tokens.glass_round);
    let inside = Edges {
        bottom: if alone { edge } else { 0.0 },
        ..Edges::all(edge)
    };
    let band = rect.pad(inside);
    let color = ctx.styles.panel_head();
    ctx.rounded(band, color, (round - edge).max(0.0));
    if alone {
        return;
    }
    let lower = Rect::new(band.x, band.y + band.h / 2.0, band.w, band.h / 2.0);
    ctx.quad(lower, color);
    let rule = Rect::new(rect.x, rect.bottom() - edge, rect.w, edge);
    ctx.quad(rule, ctx.styles.line());
}

/// A pane of glass around `content`, `pad` wider on each side and as tall as a bar. Returns the pane.
pub fn glass_pill<A: App>(ctx: &mut Ctx<'_, A>, content: Rect, pad: f32) -> Rect {
    let tall = ctx.tokens.bar;
    let top = content.y + (content.h - tall) / 2.0;
    let pill = Rect::new(content.x - pad, top, content.w + pad * 2.0, tall);
    glass_pane(ctx, pill);
    pill
}

/// A round button made of glass, `mark` in its middle, the whole glass lit under the pointer.
pub fn glass_button<A: App>(
    ctx: &mut Ctx<'_, A>,
    rect: Rect,
    (mark, role): (Mark, Role),
    target: A::Target,
) -> bool {
    let size = rect.w.min(rect.h);
    let round = rect.align((size, size), Align::Center, Align::Center);
    let radius = size / 2.0;
    ctx.glass(
        round,
        (radius, radius.min(ctx.tokens.bezel)),
        ctx.styles.glass(),
    );
    let on = ctx.interact(round, target);
    if on {
        ctx.rounded(round, ctx.styles.glass_hover(), radius);
    }
    let icon = ctx.tokens.icon;
    let at = round.align((icon, icon), Align::Center, Align::Center);
    let color = ctx.styles.color(role);
    ctx.icon(at, mark, 0, color);
    on
}

/// `rect` on `ground`, which glass drawn over it shows blurred.
pub fn ground_under<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect, ground: Ground) {
    ctx.under(rect, ground.color(&ctx.styles));
}

/// The theme's grid over `rect`, laid with its ground so that all else stands over it: a hairline
/// every `GRID`, counted from the window's corner so it never moves. Nothing for a theme with none.
pub fn ground_grid<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect) {
    let Some(color) = ctx.styles.grid() else {
        return;
    };
    let pitch = (crate::base::tokens::GRID * ctx.tokens.scale)
        .round()
        .max(1.0);
    let edge = ctx.tokens.hairline;
    let mut x = (rect.x / pitch).ceil() * pitch;
    while x < rect.right() {
        ctx.under(Rect::new(x, rect.y, edge, rect.h), color);
        x += pitch;
    }
    let mut y = (rect.y / pitch).ceil() * pitch;
    while y < rect.bottom() {
        ctx.under(Rect::new(rect.x, y, rect.w, edge), color);
        y += pitch;
    }
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
