//! What stands left of the text: the number cells and the rule that ends them.

use groove_gfx::Rect;

use super::Gutters;
use crate::ctx::Ctx;
use crate::style::Role;

/// The gutter block, measured once for the surface.
#[derive(Debug, Clone, Copy)]
pub(super) struct Block {
    cells: usize,
    pub(super) width: f32,
}

impl Block {
    pub(super) fn of(ctx: &mut Ctx, gutters: Gutters) -> Self {
        let numbers = ctx.styles.code(Role::Ghost);
        let widest = "0".repeat(gutters.digits);
        Self {
            cells: gutters.cells,
            width: ctx.measure(&widest, &numbers),
        }
    }

    /// Where a line's text starts, past every gutter and the hairline.
    pub(super) fn content(&self, ctx: &Ctx, rect: Rect) -> f32 {
        if self.cells == 0 {
            return rect.x + ctx.tokens.md;
        }
        let small = ctx.tokens.sm;
        let block = (self.width + small) * self.cells as f32;
        rect.x + small + block + ctx.tokens.md
    }
}

/// The hairline the gutters end at, down the whole surface.
pub(super) fn rule(ctx: &mut Ctx, rect: Rect, gutter: Block) {
    if gutter.cells == 0 {
        return;
    }
    let (color, thickness) = (ctx.styles.line(), ctx.tokens.hairline);
    let at = gutter.content(ctx, rect) - ctx.tokens.md;
    ctx.quad(Rect::new(at, rect.y, thickness, rect.h), color);
}
