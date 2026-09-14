use groove_gfx::Rect;

use crate::ctx::Ctx;
use crate::mark::Mark;
use crate::style::Role;
use crate::widget::{box_in, row};

/// Icon and number pairs from `x`, left to right, in the row's middle. A count of
/// zero is not drawn. Returns the x after the last pair.
pub fn counts(ctx: &mut Ctx, line: Rect, x: f32, items: &[(Mark, u32, Role)]) -> f32 {
    let gap = ctx.tokens.xs;
    let mut at = x;
    for (mark, value, role) in items.iter().filter(|(_, value, _)| *value > 0) {
        let style = ctx.styles.small(*role);
        let color = ctx.styles.color(*role);
        let box_ = box_in(line, at, style.size);
        ctx.icon(box_, *mark, 0, color);
        at += style.size + gap;
        let text = value.to_string();
        let width = ctx.measure(&text, &style);
        row(ctx, Rect::new(at, line.y, width, line.h), 0.0, &text, style);
        at += width + ctx.tokens.md;
    }
    at
}
