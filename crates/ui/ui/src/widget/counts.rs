use groove_gfx::Rect;

use crate::ctx::Ctx;
use crate::mark::Mark;
use crate::style::Role;
use crate::widget::{box_in, row};

/// The room `counts` needs, for a caller placing it against a right edge.
pub fn counts_room(ctx: &mut Ctx, items: &[(Mark, u32, Role)]) -> f32 {
    let mut wide = 0.0;
    for (_, value, role) in items.iter().filter(|(_, value, _)| *value > 0) {
        let style = ctx.styles.small(*role);
        wide += style.size + ctx.tokens.xs;
        wide += ctx.measure(&value.to_string(), &style) + ctx.tokens.md;
    }
    wide
}

/// Icon and number pairs from `x`, zeros left out; returns the x after the last.
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
