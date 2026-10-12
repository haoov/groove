use groove_gfx::{Rect, TextStyle};

use crate::base::ctx::{App, Ctx};
use crate::base::style::Role;
use crate::shape::hairline;
use crate::text::row;

/// A line of typed text with a caret after it, a hairline under it.
pub fn input<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect, prefix: &str, text: &str) {
    input_at(ctx, rect, prefix, (text, text));
}

/// The same, its caret after `before`, the part of `text` that stands before it.
pub fn input_at<A: App>(
    ctx: &mut Ctx<'_, A>,
    rect: Rect,
    prefix: &str,
    (text, before): (&str, &str),
) {
    let style = ctx.styles.body(Role::Text);
    let typed = format!("{prefix}{text}");
    let pad = ctx.tokens.md;
    row(ctx, rect, pad, &typed, style);
    caret_after(ctx, rect, pad, &format!("{prefix}{before}"), style);

    let line = ctx.styles.line();
    hairline(ctx, rect, line);
}

/// The caret of a line typed in `style`, `indent` into `rect`, after `before`.
pub fn caret_after<A: App>(
    ctx: &mut Ctx<'_, A>,
    rect: Rect,
    indent: f32,
    before: &str,
    style: TextStyle,
) {
    let width = ctx.measure(before, &style);
    let (thickness, height) = (ctx.tokens.hairline * 2.0, style.size);
    let caret = Rect::new(
        rect.x + indent + width,
        rect.y + (rect.h - height) / 2.0,
        thickness,
        height,
    );
    let color = ctx.styles.color(Role::Accent);
    ctx.quad(caret, color);
}
