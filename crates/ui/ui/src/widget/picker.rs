use groove_gfx::Rect;

use crate::ctx::Ctx;
use crate::mark::Mark;
use crate::style::Role;
use crate::widget::{box_in, row};

/// A value the user can change, with the caret that says so. Returns the x after it.
pub fn picker(ctx: &mut Ctx, line: Rect, x: f32, label: &str, role: Role) -> f32 {
    let style = ctx.styles.body(role);
    let width = ctx.measure(label, &style);
    row(ctx, Rect::new(x, line.y, width, line.h), 0.0, label, style);

    let mut at = x + width + ctx.tokens.xs;
    let color = ctx.styles.color(Role::Faint);
    let caret = box_in(line, at, style.size);
    ctx.icon(caret, Mark::Down, 0, color);
    at += style.size;
    at
}

/// A hairline standing between two things. Returns the x after it.
pub fn divider(ctx: &mut Ctx, line: Rect, x: f32) -> f32 {
    let (thickness, height) = (ctx.tokens.hairline, ctx.tokens.lg);
    let color = ctx.styles.line();
    let rect = Rect::new(x, line.y + (line.h - height) / 2.0, thickness, height);
    ctx.quad(rect, color);
    x + thickness
}
