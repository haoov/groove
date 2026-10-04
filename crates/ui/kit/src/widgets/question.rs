//! A question in the place of what asked it, and the one-word offers a row makes.

use groove_gfx::{Edges, Rect};

use crate::base::ctx::{App, Ctx};
use crate::base::ground::Ground;
use crate::base::style::Role;
use crate::text::Label;
use crate::widgets::Button;

/// One word at the right of `room`, with a ground of its own under the pointer.
pub fn offer<A: App>(ctx: &mut Ctx<'_, A>, room: &mut Rect, label: &str, target: A::Target) {
    let (ground, action) = (ctx.styles.ground(), ctx.styles.action());
    let word = Button::new(label, target, Role::Muted, ground)
        .flat()
        .hover(action);
    word.right(ctx, room, ctx.tokens.sm);
}

/// The question across `line`, its answers at its end, the last of them rightmost.
pub fn question<A: App>(
    ctx: &mut Ctx<'_, A>,
    line: Rect,
    question: &str,
    answers: [(&str, A::Target); 2],
) {
    crate::shape::ground(ctx, line, Ground::Raised);
    let mut room = line;
    for (label, target) in answers.into_iter().rev() {
        offer(ctx, &mut room, label, target);
    }
    let asked = room.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
    Label::new(question, ctx.styles.small(Role::Bad)).draw(ctx, asked);
}
