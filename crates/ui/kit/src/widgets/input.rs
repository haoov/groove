use groove_gfx::{Rect, TextStyle};

use crate::base::ctx::{App, Ctx};
use crate::base::style::Role;
use crate::shape::hairline;
use crate::text::row;
use crate::widgets::{caret_bar, text_band};

/// A line of typed text with its caret after `before`, the part of `text` before it; a hairline under it.
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

/// An input line led by the step it answers, dimmed; `hint` faint at its right end.
pub fn stepped_input<A: App>(
    ctx: &mut Ctx<'_, A>,
    rect: Rect,
    step: &str,
    (text, before, hint): (&str, &str, Option<&str>),
) {
    let (style, faint) = (ctx.styles.body(Role::Text), ctx.styles.body(Role::Faint));
    let pad = ctx.tokens.md;
    row(ctx, rect, pad, step, faint);
    let at = pad + ctx.measure(step, &faint) + pad;
    row(ctx, rect, at, text, style);
    if let Some(hint) = hint {
        let x = rect.w - pad - ctx.measure(hint, &faint);
        row(ctx, rect, x, hint, faint);
    }
    caret_after(ctx, rect, at, before, style);
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
    let span = Rect::new(
        rect.x,
        rect.y + (rect.h - style.size) / 2.0,
        rect.w,
        style.size,
    );
    let color = ctx.styles.color(Role::Accent);
    caret_bar(ctx, rect.x + indent + width, span, color);
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
        let past = if through {
            ctx.measure(" ", &style)
        } else {
            0.0
        };
        let end = upto(ctx, to) + past;
        let color = ctx.styles.held();
        text_band(ctx, rect, (start, end), color);
    }
    row(ctx, rect, 0.0, text, style);
    if let Some(column) = caret {
        let at = rect.x + upto(ctx, column);
        let color = ctx.styles.caret();
        caret_bar(ctx, at, rect, color);
    }
}
