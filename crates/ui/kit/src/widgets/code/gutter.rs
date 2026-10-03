//! What stands left of the text: the number cells and the rule that ends them.

use groove_gfx::Rect;

use crate::base::ctx::{App, Ctx};
use crate::base::style::Role;

/// The gutter a surface asks for: how many number columns, and their longest number.
#[derive(Debug, Clone, Copy)]
pub struct Gutters {
    pub cells: usize,
    pub digits: usize,
}

/// The gutter block, measured once for the surface.
#[derive(Debug, Clone, Copy)]
pub struct Block {
    cells: usize,
    pub width: f32,
}

impl Block {
    pub fn of<A: App>(ctx: &mut Ctx<'_, A>, gutters: Gutters) -> Self {
        let numbers = ctx.styles.code(Role::Ghost);
        let widest = "0".repeat(gutters.digits);
        Self {
            cells: gutters.cells,
            width: ctx.measure(&widest, &numbers),
        }
    }

    /// The rect number cell `at` of a row in `line` stands in.
    pub fn cell<A: App>(&self, ctx: &Ctx<'_, A>, line: Rect, at: usize) -> Rect {
        let small = ctx.tokens.sm;
        let x = line.x + small + at as f32 * (self.width + small);
        Rect::new(x, line.y, self.width, line.h)
    }

    /// Where the last number cell ends, which every number is right-aligned to.
    pub fn numbers_end<A: App>(&self, ctx: &Ctx<'_, A>, rect: Rect) -> f32 {
        if self.cells == 0 {
            return rect.x;
        }
        let small = ctx.tokens.sm;
        let before = (self.width + small) * (self.cells - 1) as f32;
        rect.x + small + before + self.width
    }

    /// Where a line's text starts, past every gutter and the hairline.
    pub fn content<A: App>(&self, ctx: &Ctx<'_, A>, rect: Rect) -> f32 {
        if self.cells == 0 {
            return rect.x;
        }
        let small = ctx.tokens.sm;
        let block = (self.width + small) * self.cells as f32;
        rect.x + small + block + ctx.tokens.md
    }
}

/// The hairline the gutters end at, down the whole surface.
pub fn rule<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect, gutter: Block) {
    if gutter.cells == 0 {
        return;
    }
    let (color, thickness) = (ctx.styles.line(), ctx.tokens.hairline);
    let at = gutter.content(ctx, rect) - ctx.tokens.md;
    ctx.quad(Rect::new(at, rect.y, thickness, rect.h), color);
}
