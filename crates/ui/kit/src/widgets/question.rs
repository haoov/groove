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

/// A count the question sets before it is answered, between its step down and its step up.
pub struct Steps<'a, T> {
    pub shown: &'a str,
    pub less: Option<T>,
    pub more: T,
}

/// The question across the whole of `line`, square: its count when it sets one, then its answers.
pub fn question<A: App>(
    ctx: &mut Ctx<'_, A>,
    line: Rect,
    (question, steps): (&str, Option<Steps<'_, A::Target>>),
    answers: [(&str, A::Target); 2],
) {
    let tint = ctx.styles.band_tint(Role::Attention);
    Panel::default().ground(tint).draw(ctx, line);
    let mut room = line.pad(Edges::xy(ctx.tokens.md, ctx.tokens.xs / 2.0));
    let asked = Label::new(question, ctx.styles.small(Role::Attention));
    asked.left(ctx, &mut room, ctx.tokens.md);
    if let Some(steps) = steps {
        stepped(ctx, &mut room, steps);
    }
    let [(yes, said), (no, kept)] = answers;
    let edge = ctx.styles.band_edge(Role::Attention);
    let gap = ctx.tokens.sm;
    Button::new(yes, said, Role::Attention, tint)
        .edged(edge)
        .left(ctx, &mut room, gap);
    let raised = ctx.styles.panel_head();
    Button::new(no, kept, Role::Muted, raised).left(ctx, &mut room, gap);
}

/// `− 2 +`, the step down left out when there is none, a wider gap after it.
fn stepped<A: App>(ctx: &mut Ctx<'_, A>, room: &mut Rect, steps: Steps<'_, A::Target>) {
    let (sm, raised) = (ctx.tokens.sm, ctx.styles.panel_head());
    if let Some(less) = steps.less {
        Button::new("−", less, Role::Muted, raised).left(ctx, room, sm);
    }
    let shown = Label::new(steps.shown, ctx.styles.code(Role::Text));
    shown.left(ctx, room, sm);
    Button::new("+", steps.more, Role::Muted, raised).left(ctx, room, ctx.tokens.md);
}
