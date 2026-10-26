//! A question in the place of what asked it, and the one-word offers a row makes.

use groove_gfx::{Edges, Rect};

use crate::base::ctx::{App, Ctx};
use crate::base::style::Role;
use crate::text::Label;
use crate::widgets::Button;

/// One word at the right of `room`, on a pill of glass under the pointer.
pub fn offer<A: App>(ctx: &mut Ctx<'_, A>, room: &mut Rect, label: &str, target: A::Target) {
    let word = Button::glass(label, target, Role::Muted, &ctx.styles).flat();
    word.right(ctx, room, ctx.tokens.sm);
}

/// The question across `line`, its answers at its end, the last of them rightmost.
pub fn question<A: App>(
    ctx: &mut Ctx<'_, A>,
    line: Rect,
    question: &str,
    answers: [(&str, A::Target); 2],
) {
    let card = line.pad(Edges::xy(ctx.tokens.sm, ctx.tokens.xs / 2.0));
    ctx.rounded(card, ctx.styles.glass_card(), ctx.tokens.sm);
    let mut room = card.pad(Edges::across(0.0, ctx.tokens.xs));
    for (label, target) in answers.into_iter().rev() {
        offer(ctx, &mut room, label, target);
    }
    let asked = room.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
    Label::new(question, ctx.styles.small(Role::Bad)).draw(ctx, asked);
}
