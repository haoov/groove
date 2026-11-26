//! The commands `/` offers in the prompt: the agent's, then Groove's, each group under its head.

use groove_gfx::{Edges, Rect};
use groove_ui_kit::base::style::Role;
use groove_ui_kit::base::tokens::COMPLETION_ROWS;
use groove_ui_kit::shape::{from_left, from_top, hairline};
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{Offer, glass_chosen, glass_hoverable};

use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::widgets::cap;

/// The offers in sight around the `chosen` one, and how many group heads stand among them.
fn window(offers: &[Offer], own: usize, chosen: usize) -> (std::ops::Range<usize>, usize) {
    let shown = COMPLETION_ROWS.min(offers.len());
    let chosen = chosen.min(offers.len().saturating_sub(1));
    let first = chosen
        .saturating_sub(shown.saturating_sub(1))
        .min(offers.len() - shown);
    let window = first..first + shown;
    let heads = [0, own]
        .into_iter()
        .filter(|at| window.contains(at))
        .count();
    let heads = heads - usize::from(own == 0 && window.contains(&0));
    (window, heads)
}

/// How tall the offers stand over the typed rows; nothing without one.
pub(super) fn height(ctx: &Ctx, (offers, own): (&[Offer], usize), chosen: usize) -> f32 {
    if offers.is_empty() {
        return 0.0;
    }
    let (window, heads) = window(offers, own, chosen);
    (window.len() + heads) as f32 * ctx.tokens.glass_row + ctx.tokens.sm * 2.0
}

/// The offers in `rect`, the agent's `own` first, each group under its head, ruled off under.
pub(super) fn draw(ctx: &mut Ctx, rect: Rect, (offers, own): (&[Offer], usize), chosen: usize) {
    if offers.is_empty() {
        return;
    }
    hairline(ctx, rect, ctx.styles.panel_edge());
    let (window, _) = window(offers, own, chosen);
    let row = ctx.tokens.glass_row;
    let mut room = rect.pad(Edges::xy(0.0, ctx.tokens.sm));
    let widest = name_width(ctx, &offers[window.clone()]);
    for at in window {
        if at == own || (at == 0 && own > 0) {
            let (word, count) = match at < own {
                true => ("the agent", own),
                false => ("groove", offers.len() - own),
            };
            let line = from_top(&mut room, row, 0.0);
            let mut head = line.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
            cap(ctx, &mut head, &format!("{word} · {count}"), Role::Ghost);
        }
        let line = from_top(&mut room, row, 0.0);
        offer(
            ctx,
            line,
            (&offers[at], widest),
            (at, at == chosen.min(offers.len() - 1)),
        );
    }
}

fn name_width(ctx: &mut Ctx, offers: &[Offer]) -> f32 {
    let style = ctx.styles.code(Role::Text);
    let widths = offers
        .iter()
        .map(|one| ctx.measure(&format!("/{}", one.label), &style));
    widths.collect::<Vec<_>>().into_iter().fold(0.0, f32::max)
}

/// One command: `/name` in mono, then what it does.
fn offer(ctx: &mut Ctx, line: Rect, (one, widest): (&Offer, f32), (at, chosen): (usize, bool)) {
    if chosen {
        glass_chosen(ctx, line);
    }
    glass_hoverable(ctx, line, Target::Offered(at));
    let mut room = line.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
    let name = format!("/{}", one.label);
    let slot = from_left(&mut room, widest, ctx.tokens.lg);
    Label::new(&name, ctx.styles.code(Role::Text)).draw(ctx, slot);
    Label::new(one.detail, ctx.styles.small(Role::Faint)).draw(ctx, room);
}
