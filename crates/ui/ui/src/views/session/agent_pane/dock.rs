//! The agents still at work, one row each on glass over the prompt.

use groove_gfx::{Edges, Rect};
use groove_types::chat::{Sent, Subagent};
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::{band, from_top};
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{fold, glass_hoverable, glass_pane};

use super::subagent;
use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::widgets::{cap, cap_right};

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
    let tall = (rows + 1) as f32 * ctx.tokens.row;
    Some(band(pane, under - pane.y - ctx.tokens.sm - tall, tall))
}

/// `AGENTS · 1 WORKING · 1 DONE` and its chevron, then each agent's row.
pub(super) fn draw(ctx: &mut Ctx, dock: Rect, docked: &[&Subagent], folded: bool) {
    glass_pane(ctx, dock);
    let mut room = dock;
    let top = from_top(&mut room, ctx.tokens.row, 0.0);
    glass_hoverable(ctx, top, Target::FoldAgents);
    let mut top = top.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
    fold(ctx, &mut top, !folded, Role::Ghost);
    cap(ctx, &mut top, &counts(docked), Role::Muted);
    if folded {
        return;
    }
    for one in docked {
        let line = from_top(&mut room, ctx.tokens.row, 0.0);
        agent(ctx, line, one);
    }
}

/// `agents · 2 working · 1 done`, by each state some agent stands in.
fn counts(docked: &[&Subagent]) -> String {
    let states = [Sent::Working, Sent::Done, Sent::Failed, Sent::Stopped];
    let counted = states.map(|state| {
        (
            docked.iter().filter(|one| one.state == state).count(),
            state,
        )
    });
    let shown = counted.iter().filter(|(count, _)| *count > 0);
    let parts = shown.map(|(count, state)| format!(" · {count} {}", subagent::state(*state).0));
    std::iter::once("agents".to_string()).chain(parts).collect()
}

/// One agent: its type and task, how it stands and its time; a click opens its block.
fn agent(ctx: &mut Ctx, line: Rect, sub: &Subagent) {
    glass_hoverable(ctx, line, Target::Subagent(sub.id.clone()));
    let mut room = line.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
    if let Some(took) = subagent::took(ctx, sub) {
        Label::new(&took, ctx.styles.log(Role::Ghost)).right(ctx, &mut room, ctx.tokens.sm);
    }
    let (word, role) = subagent::state(sub.state);
    cap_right(ctx, &mut room, word, role);
    let kind = format!("{} ·", sub.kind);
    Label::new(&kind, ctx.styles.body(Role::Text)).left(ctx, &mut room, ctx.tokens.xs);
    Label::new(&sub.task, ctx.styles.body(Role::Faint)).draw(ctx, room);
}
