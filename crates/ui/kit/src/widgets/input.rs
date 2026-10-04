use groove_gfx::Rect;

use crate::base::ctx::{App, Ctx};
use crate::base::style::Role;
use crate::shape::hairline;
use crate::text::row;

/// A line of typed text with a caret after it, a hairline under it.
pub fn input<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect, prefix: &str, text: &str) {
    let style = ctx.styles.body(Role::Text);
    let typed = format!("{prefix}{text}");
    let pad = ctx.tokens.md;
    row(ctx, rect, pad, &typed, style);

    let width = ctx.measure(&typed, &style);
    let (thickness, height) = (ctx.tokens.hairline * 2.0, style.size);
    let caret = Rect::new(
        rect.x + pad + width,
        rect.y + (rect.h - height) / 2.0,
        thickness,
        height,
    );
    let color = ctx.styles.color(Role::Accent);
    ctx.quad(caret, color);

    let line = ctx.styles.line();
    hairline(ctx, rect, line);
}
