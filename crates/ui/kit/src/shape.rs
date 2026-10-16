//! Placement and rules: where a box sits in a row, and the lines drawn around one.

use groove_gfx::{Align, Color, Rect};

use crate::base::ctx::{App, Ctx};
use crate::base::ground::Ground;

/// A box of `size` with its top-left at `at`, moved back inside `within` where it would leave it.
pub fn kept_in(at: (f32, f32), (width, height): (f32, f32), within: Rect) -> Rect {
    let x = at.0.min(within.right() - width).max(within.x);
    let y = at.1.min(within.bottom() - height).max(within.y);
    Rect::new(x, y, width, height)
}

/// A square box of `size`, centred vertically in `row`, at `x`.
pub fn box_in(row: Rect, x: f32, size: f32) -> Rect {
    Rect::new(x, row.y + (row.h - size) / 2.0, size, size)
}

/// A box of `size` hanging under `rect`, at its left.
pub fn hung_under(rect: Rect, (width, height): (f32, f32)) -> Rect {
    Rect::new(rect.x, rect.bottom(), width, height)
}

/// A square of `size` centred down `slot`, at its start.
pub fn square(slot: Rect, size: f32) -> Rect {
    slot.align((size, size), Align::Start, Align::Center)
}

/// A leading icon in `row` at `x`, the icon size.
pub fn leading<A: App>(ctx: &Ctx<'_, A>, row: Rect, x: f32) -> Rect {
    box_in(row, x, ctx.tokens.icon)
}

/// Where a row's text starts when a mark leads it at `indent`.
pub fn after_mark<A: App>(ctx: &Ctx<'_, A>, indent: f32) -> f32 {
    indent + ctx.tokens.icon + ctx.tokens.sm
}

/// A hairline along the top edge of `rect`, inside it.
pub fn top_rule<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect, color: Color) {
    ctx.quad(
        Rect::new(rect.x, rect.y, rect.w, ctx.tokens.hairline),
        color,
    );
}

/// A hairline just above `rect`, outside it.
pub fn rule_above<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect, color: Color) {
    let thickness = ctx.tokens.hairline;
    ctx.quad(
        Rect::new(rect.x, rect.y - thickness, rect.w, thickness),
        color,
    );
}

/// A hairline down `rect`'s height, its left side at `x`.
pub fn side_rule<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect, x: f32, color: Color) {
    ctx.quad(Rect::new(x, rect.y, ctx.tokens.hairline, rect.h), color);
}

/// A hairline above `rect` and one below it.
pub fn ruled<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect, color: Color) {
    let thickness = ctx.tokens.hairline;
    ctx.quad(Rect::new(rect.x, rect.y, rect.w, thickness), color);
    let under = rect.bottom() - thickness;
    ctx.quad(Rect::new(rect.x, under, rect.w, thickness), color);
}

/// What a list marks as the one it stands on: the held ground between the chosen rules.
pub fn chosen<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect) {
    chosen_ground(ctx, rect);
    chosen_rules(ctx, rect);
}

/// The ground of [`chosen`] alone.
pub fn chosen_ground<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect) {
    ground(ctx, rect, Ground::Held);
}

/// The rules of [`chosen`].
pub fn chosen_rules<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect) {
    ruled(ctx, rect, ctx.styles.chosen());
}

/// The focused region's mark: a line of two hairlines in the accent along the bottom of `rect`.
pub fn focus_line<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect) {
    let thick = ctx.tokens.hairline * 2.0;
    let color = ctx.styles.color(crate::base::style::Role::Accent);
    ctx.quad(
        Rect::new(rect.x, rect.bottom() - thick, rect.w, thick),
        color,
    );
}

/// A hairline along the bottom of `rect`.
pub fn hairline<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect, color: Color) {
    let thickness = ctx.tokens.hairline;
    ctx.quad(
        Rect::new(rect.x, rect.bottom() - thickness, rect.w, thickness),
        color,
    );
}

/// Registers `target` at `rect`, on the hover ground while the pointer rests on it.
pub fn hoverable<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect, target: A::Target) -> bool {
    let on = ctx.interact(rect, target);
    if on {
        let hover = ctx.styles.hover();
        ctx.quad(rect, hover);
    }
    on
}

/// A box: a ground and a one-pixel border.
#[derive(Debug, Clone, Copy, Default)]
pub struct Panel {
    pub ground: Option<Color>,
    pub border: Option<Color>,
    pub radius: f32,
}

impl Panel {
    pub fn ground(mut self, color: Color) -> Self {
        self.ground = Some(color);
        self
    }

    pub fn border(mut self, color: Color) -> Self {
        self.border = Some(color);
        self
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    pub fn draw<A: App>(self, ctx: &mut Ctx<'_, A>, rect: Rect) {
        if self.radius > 0.0 {
            if let Some(ground) = self.ground {
                ctx.rounded(rect, ground, self.radius);
            }
            if let Some(border) = self.border {
                ctx.ring(rect, border, self.radius, 1.0);
            }
            return;
        }
        if let Some(ground) = self.ground {
            ctx.quad(rect, ground);
        }
        if let Some(border) = self.border {
            ctx.border(rect, border);
        }
    }
}

pub fn ground<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect, ground: Ground) {
    let color = ground.color(&ctx.styles);
    ctx.quad(rect, color);
}

/// A shade fading down from under `band`, onto what it stands over.
pub fn shadow_under<A: App>(ctx: &mut Ctx<'_, A>, band: Rect) {
    let blur = ctx.tokens.sm;
    let rect = Rect::new(
        band.x - blur,
        band.bottom() - blur,
        band.w + blur * 2.0,
        blur * 2.0,
    );
    let color = ctx.styles.shadow();
    ctx.shadow(rect, color, blur);
}

/// One row of a card: its ground and its sides, and its top or bottom edge where it has one.
pub fn card_row<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect, edges: (bool, bool)) {
    let (ground, line) = (ctx.styles.band(), ctx.styles.line());
    ctx.quad(rect, ground);
    framed(ctx, rect, edges, line);
}

/// One row of a frame in `line`: its sides, and its top or bottom edge where it has one.
pub fn framed<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect, (top, bottom): (bool, bool), line: Color) {
    let thickness = ctx.tokens.hairline;
    ctx.quad(Rect::new(rect.x, rect.y, thickness, rect.h), line);
    ctx.quad(
        Rect::new(rect.right() - thickness, rect.y, thickness, rect.h),
        line,
    );
    if top {
        ctx.quad(Rect::new(rect.x, rect.y, rect.w, thickness), line);
    }
    if bottom {
        ctx.quad(
            Rect::new(rect.x, rect.bottom() - thickness, rect.w, thickness),
            line,
        );
    }
}
