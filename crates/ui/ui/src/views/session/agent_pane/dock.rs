//! The agents still at work, on glass over the prompt: one row each, so none is lost while the conversation goes on.

use groove_gfx::{Edges, Rect};
use groove_types::chat::{Sent, Subagent};
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::base::tokens::DOING_SHARE;
use groove_ui_kit::shape::{band, from_right, from_top};
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{Button, glass_hoverable, glass_pane};

use super::subagent;
use crate::ctx::Ctx;
use crate::hit::Target;

/// The dock standing on `under`, its rows or only its head when `folded`; none without an agent.
pub(super) fn laid(
    ctx: &Ctx,
    docked: &[&Subagent],
    folded: bool,
    under: f32,
    pane: Rect,
) -> Option<Rect> {
    if docked.is_empty() {
        return None;
    }
    let rows = if folded { 0 } else { docked.len() };
    let (row, sm) = (ctx.tokens.row, ctx.tokens.sm);
    let tall = (rows + 1) as f32 * row + sm * 2.0;
    Some(band(pane, under - pane.y - sm - tall, tall))
}

/// Its head, how many work and how many are done, then each agent's row.
pub(super) fn draw(ctx: &mut Ctx, dock: Rect, docked: &[&Subagent], folded: bool) {
    glass_pane(ctx, dock);
    let mut room = dock.pad(Edges::xy(ctx.tokens.md, ctx.tokens.sm));
    let mut head = from_top(&mut room, ctx.tokens.row, 0.0);
    let turn = if folded { 2 } else { 0 };
    let fold = Button::icon(Mark::Down, turn, Target::FoldAgents, Role::Faint).flat();
    fold.right(ctx, &mut head, ctx.tokens.sm);
    Label::new("agents", ctx.styles.small(Role::Ghost)).left(ctx, &mut head, ctx.tokens.sm);
    Label::new(&counts(docked), ctx.styles.small(Role::Faint)).draw(ctx, head);
    if folded {
        return;
    }
    for one in docked {
        let line = from_top(&mut room, ctx.tokens.row, 0.0);
        agent(ctx, line, one);
    }
}

/// `2 working · 1 done`, by each state some agent stands in.
fn counts(docked: &[&Subagent]) -> String {
    let states = [Sent::Working, Sent::Done, Sent::Failed, Sent::Stopped];
    let counted = states.map(|state| {
        (
            docked.iter().filter(|one| one.state == state).count(),
            state,
        )
    });
    let shown = counted.iter().filter(|(count, _)| *count > 0);
    let parts: Vec<String> = shown
        .map(|(count, state)| format!("{count} {}", subagent::state(*state).0))
        .collect();
    parts.join(" · ")
}

/// One agent: its header as on its card, what it does now under the pointer's click to its card.
fn agent(ctx: &mut Ctx, line: Rect, sub: &Subagent) {
    glass_hoverable(ctx, line, Target::Subagent(sub.id.clone()));
    let mut line = line.pad(Edges::across(ctx.tokens.xs, ctx.tokens.xs));
    if let Some(doing) = sub
        .steps
        .doing
        .as_deref()
        .filter(|_| sub.state == Sent::Working)
    {
        let wide = line.w * DOING_SHARE;
        let tail = from_right(&mut line, wide, ctx.tokens.sm);
        Label::new(doing, ctx.styles.small(Role::Faint)).draw(ctx, tail);
    }
    subagent::header(ctx, line, sub);
}
