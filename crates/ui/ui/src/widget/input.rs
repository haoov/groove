use groove_gfx::Rect;

use crate::ctx::Ctx;
use crate::style::Role;
use crate::widget::{hairline, row};

const CARET: &str = "▏";

/// A line of typed text with a caret, a hairline under it.
pub fn input(ctx: &mut Ctx, rect: Rect, prefix: &str, text: &str) {
    let style = ctx.styles.mono(Role::Text);
    let line = format!("{prefix}{text}{CARET}");
    row(ctx, rect, ctx.tokens.md, &line, style);
    let color = ctx.styles.line();
    hairline(ctx, rect, color);
}
