//! Placement and rules: where a box sits in a row, and the lines drawn around one.

use groove_gfx::{Align, Color, Rect};

use crate::base::ctx::{App, Ctx};

/// A square box of `size`, centred vertically in `row`, at `x`.
pub fn box_in(row: Rect, x: f32, size: f32) -> Rect {
    Rect::new(x, row.y + (row.h - size) / 2.0, size, size)
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

/// A hairline above `rect` and one below it.
pub fn ruled<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect, color: Color) {
    let thickness = ctx.tokens.hairline;
    ctx.quad(Rect::new(rect.x, rect.y, rect.w, thickness), color);
    let under = rect.bottom() - thickness;
    ctx.quad(Rect::new(rect.x, under, rect.w, thickness), color);
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

    pub fn draw<A: App>(self, ctx: &mut Ctx<'_, A>, rect: Rect) {
        if let Some(ground) = self.ground {
            ctx.quad(rect, ground);
        }
        if let Some(border) = self.border {
            ctx.border(rect, border);
        }
    }
}
