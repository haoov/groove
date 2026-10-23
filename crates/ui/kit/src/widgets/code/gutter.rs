//! What stands left of the text: the number cells and the rule that ends them.

use groove_gfx::Rect;

use super::Tile;
use crate::base::ctx::{App, Ctx};
use crate::base::mark::Mark;
use crate::base::style::Role;
use crate::text::row;

/// The gutter a surface asks for: how many number columns, and their longest number.
#[derive(Debug, Clone, Copy)]
pub struct Gutters {
    pub cells: usize,
    pub digits: usize,
    /// A column before the numbers for the marks a line carries.
    pub marks: bool,
    /// A column after the numbers for the caret that folds a block.
    pub folds: bool,
}

/// The gutter block, measured once for the surface.
#[derive(Debug, Clone, Copy)]
pub struct Block {
    cells: usize,
    pub width: f32,
    /// The marks' column, before the numbers; none when the surface has no marks.
    pub mark: f32,
    /// The folds' column, after the numbers; none when the surface folds nothing.
    pub fold: f32,
}

impl Block {
    pub fn of<A: App>(ctx: &mut Ctx<'_, A>, gutters: Gutters) -> Self {
        let numbers = ctx.styles.code(Role::Ghost);
        let widest = "0".repeat(gutters.digits);
        Self {
            cells: gutters.cells,
            width: ctx.measure(&widest, &numbers),
            mark: match gutters.marks {
                true => ctx.tokens.icon + ctx.tokens.xs,
                false => 0.0,
            },
            fold: match gutters.folds {
                true => ctx.tokens.small + ctx.tokens.sm,
                false => 0.0,
            },
        }
    }

    /// The rect number cell `at` of a row in `line` stands in.
    pub fn cell<A: App>(&self, ctx: &Ctx<'_, A>, line: Rect, at: usize) -> Rect {
        let small = ctx.tokens.sm;
        let x = line.x + small + self.mark + at as f32 * (self.width + small);
        Rect::new(x, line.y, self.width, line.h)
    }

    /// Where the last number cell ends, which every number is right-aligned to.
    pub fn numbers_end<A: App>(&self, ctx: &Ctx<'_, A>, rect: Rect) -> f32 {
        if self.cells == 0 {
            return rect.x;
        }
        let small = ctx.tokens.sm;
        let before = (self.width + small) * (self.cells - 1) as f32;
        rect.x + small + self.mark + before + self.width
    }

    /// Where a line's marks stand, before its numbers.
    pub fn marked<A: App>(&self, ctx: &Ctx<'_, A>, line: Rect) -> Rect {
        Rect::new(line.x + ctx.tokens.sm, line.y, self.mark, line.h)
    }

    /// Where a line's text starts, past every gutter and the hairline.
    pub fn content<A: App>(&self, ctx: &Ctx<'_, A>, rect: Rect) -> f32 {
        if self.cells == 0 {
            return rect.x;
        }
        let small = ctx.tokens.sm;
        let block = (self.width + small) * self.cells as f32;
        rect.x + small + self.mark + block + self.fold + ctx.tokens.md
    }

    /// Where a line's fold caret stands, between its numbers and the rule.
    pub fn folding<A: App>(&self, ctx: &Ctx<'_, A>, line: Rect) -> Rect {
        let x = self.numbers_end(ctx, line) + ctx.tokens.sm;
        Rect::new(x, line.y, ctx.tokens.small, line.h)
    }
}

/// A line's notes in its marks' column: a yellow tile counting the open ones, or a tick.
pub(super) fn tiled<A: App>(
    ctx: &mut Ctx<'_, A>,
    room: Rect,
    tile: Tile,
    opens: Option<A::Target>,
) {
    let size = ctx.tokens.icon;
    let box_ = crate::shape::box_in(room, room.x, size);
    match tile {
        Tile::Open(count) => {
            let ground = ctx.styles.color(Role::Warn);
            ctx.rounded(box_, ground, ctx.tokens.round);
            let style = ctx.styles.initials(Role::Inverse);
            let text = count.to_string();
            let width = ctx.measure(&text, &style);
            row(ctx, box_, (box_.w - width) / 2.0, &text, style);
        }
        Tile::Resolved => ctx.icon(box_, Mark::Read, 0, ctx.styles.color(Role::Faint)),
    }
    if let Some(target) = opens {
        ctx.hit(box_, target);
    }
}
