//! A question in the place of what asked it, and the one-word offers a row makes.

use groove_gfx::{Edges, Rect};

use crate::base::ctx::{App, Ctx};
use crate::base::style::Role;
use crate::shape::Panel;
use crate::text::Label;
use crate::widgets::Button;

/// One word at the right of `room`, on a pill of glass under the pointer.
pub fn offer<A: App>(ctx: &mut Ctx<'_, A>, room: &mut Rect, label: &str, target: A::Target) {
    let word = Button::glass(label, target, Role::Muted, &ctx.styles).flat();
    word.right(ctx, room, ctx.tokens.sm);
}

/// The question across the whole of `line`, square, its answers right after it.
pub fn question<A: App>(
    ctx: &mut Ctx<'_, A>,
    line: Rect,
    question: &str,
    answers: [(&str, A::Target); 2],
) {
    let ground = ctx.styles.glass_card();
    Panel::default().ground(ground).draw(ctx, line);
    let mut room = line.pad(Edges::xy(ctx.tokens.md, ctx.tokens.xs / 2.0));
    let asked = Label::new(question, ctx.styles.small(Role::Bad));
    asked.left(ctx, &mut room, ctx.tokens.md);
    let [(yes, said), (no, kept)] = answers;
    let tint = ctx.styles.band_tint(Role::Bad);
    let edge = ctx.styles.band_edge(Role::Bad);
    let gap = ctx.tokens.sm;
    Button::new(yes, said, Role::Bad, tint)
        .edged(edge)
        .left(ctx, &mut room, gap);
    let raised = ctx.styles.panel_head();
    Button::new(no, kept, Role::Muted, raised).left(ctx, &mut room, gap);
}
