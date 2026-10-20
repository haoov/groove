//! Scope guides, the word's occurrences, the fold caret and what a fold hides.

use groove_gfx::Rect;

use super::paint::upto;
use super::{Block, Code};
use crate::base::ctx::{App, Ctx};
use crate::base::style::Role;
use crate::widgets::text_band;

/// The guides down the line's indent, under its text, in the comment colour.
pub(super) fn guides<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect, code: &Code<'_, A::Target>) {
    let color = ctx.styles.guide();
    let thin = ctx.tokens.hairline;
    for column in code.outline.guides {
        let x = rect.x + *column as f32 * ctx.advance;
        ctx.quad(Rect::new(x, rect.y, thin, rect.h), color);
    }
}

/// The word the caret rests in, wherever the line holds it.
pub(super) fn occurs<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect, code: &Code<'_, A::Target>) {
    let (style, color) = (ctx.styles.source(Role::Text), ctx.styles.occurs());
    for (from, to) in code.outline.occurs {
        let start = upto(ctx, code.text, *from, &style);
        let end = upto(ctx, code.text, *to, &style);
        text_band(ctx, rect, (start, end), color);
    }
}

/// The caret of the block the line opens, down while it is open and right once folded.
pub(super) fn caret<A: App>(
    ctx: &mut Ctx<'_, A>,
    line: Rect,
    code: &Code<'_, A::Target>,
    gutter: Block,
) {
    let Some((folded, _, target)) = code.outline.fold.as_ref() else {
        return;
    };
    let room = gutter.folding(ctx, line);
    let mut slot = room;
    crate::widgets::caret(ctx, &mut slot, (!folded, ctx.tokens.small), Role::Faint);
    ctx.hit(room, target.clone());
}

/// How many lines a folded block hides, after the line's text.
pub(super) fn hidden<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect, code: &Code<'_, A::Target>) {
    let Some((true, lines, _)) = code.outline.fold.as_ref() else {
        return;
    };
    let style = ctx.styles.source(Role::Text);
    let end = rect.x + upto(ctx, code.text, code.text.chars().count(), &style);
    let said = match lines {
        1 => "⋯ 1 line".to_string(),
        many => format!("⋯ {many} lines"),
    };
    crate::widgets::Badge::new(&said, Role::Faint).at(ctx, rect, end + ctx.tokens.md);
}
