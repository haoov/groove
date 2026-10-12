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

/// A buffer's line in `style`: what is held under it, its text, the caret over it.
pub fn typed_line<A: App>(
    ctx: &mut Ctx<'_, A>,
    rect: Rect,
    (text, style): (&str, TextStyle),
    (caret, held): (Option<usize>, Option<(usize, usize, bool)>),
) {
    let upto = |ctx: &mut Ctx<'_, A>, column: usize| {
        let before: String = text.chars().take(column).collect();
        ctx.measure(&before, &style)
    };
    if let Some((from, to, through)) = held {
        let start = upto(ctx, from);
        let end = upto(ctx, to) + if through { ctx.advance } else { 0.0 };
        let band = Rect::new(rect.x + start, rect.y, (end - start).max(1.0), rect.h);
        let color = ctx.styles.held();
        ctx.quad(band, color);
    }
    row(ctx, rect, 0.0, text, style);
    if let Some(column) = caret {
        let at = rect.x + upto(ctx, column);
        let (width, color) = (ctx.tokens.hairline * 2.0, ctx.styles.caret());
        ctx.quad(Rect::new(at, rect.y, width, rect.h), color);
    }
}
