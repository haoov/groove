use groove_gfx::Rect;

use crate::ctx::Ctx;
use crate::mark::Mark;
use crate::style::Role;
use crate::widget::{box_in, row, slot_at};

/// A value the user can change, as a button with the caret that says so. Returns what
/// it covers.
pub fn picker(ctx: &mut Ctx, line: Rect, x: f32, label: &str, role: Role, lit: bool) -> Rect {
    let style = ctx.styles.body(role);
    let width = ctx.measure(label, &style);
    let ground = match lit {
        true => ctx.styles.hover(),
        false => ctx.styles.band(),
    };
    let box_ = slot_at(
        ctx,
        line,
        x,
        width + ctx.tokens.xs + style.size,
        Some(ground),
    );
    row(ctx, box_, ctx.tokens.sm, label, style);
    let at = box_.x + ctx.tokens.sm + width + ctx.tokens.xs;
    let caret = box_in(box_, at, style.size);
    ctx.icon(caret, Mark::Down, 0, ctx.styles.color(Role::Faint));
    box_
}
