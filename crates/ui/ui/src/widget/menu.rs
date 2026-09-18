//! The panel the right button opens: a row per action, at the point it was asked.

use groove_gfx::Rect;

use crate::ctx::Ctx;
use crate::hit::Target;
use crate::style::Role;
use crate::widget::row;

/// Draws the rows at `at`, kept inside `within`. Registers a hit per row.
pub fn menu(ctx: &mut Ctx, at: (f32, f32), within: Rect, labels: &[&str], hovered: Option<usize>) {
    let height = ctx.tokens.row;
    let width = ctx.tokens.menu;
    let tall = height * labels.len() as f32;
    let x = at.0.min(within.right() - width).max(within.x);
    let y = at.1.min(within.bottom() - tall).max(within.y);
    let box_ = Rect::new(x, y, width, tall);
    ctx.quad(box_, ctx.styles.panel());
    ctx.border(box_, ctx.styles.border());
    for (index, label) in labels.iter().enumerate() {
        let line = Rect::new(x, y + height * index as f32, width, height);
        if hovered == Some(index) {
            ctx.quad(line, ctx.styles.raised());
        }
        let style = ctx.styles.body(Role::Text);
        row(ctx, line, ctx.tokens.md, label, style);
        ctx.hit(line, Target::MenuRow(index));
    }
}
