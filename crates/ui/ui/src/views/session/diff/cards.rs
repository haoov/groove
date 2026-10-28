//! Read notes' cards: one pane of glass each over the rows it takes, its words drawn on it.

use groove_gfx::Rect;

use super::notes::{Slot, floats};
use super::row::Side;
use crate::components::{Block, Gutters, Line, Room, Rows, card_across, code};
use crate::ctx::Ctx;
use groove_ui_kit::widgets::glass_pane;

/// The rows drawn, then the read notes' cards over them on the new side.
pub(super) fn painted(
    ctx: &mut Ctx,
    rect: Rect,
    (lines, first, gutters, across): (&[Line<'_>], usize, Gutters, f32),
    (side, slots): (Side, &[Slot]),
    scroll: f32,
) -> Vec<Rect> {
    let shown = |floating| Rows {
        lines,
        first,
        gutters,
        across,
        floating,
    };
    let drawn = code(ctx, rect, shown(false), scroll);
    if side != Side::Old {
        draw(ctx, rect, shown(true), (slots, &drawn), scroll);
    }
    drawn
}

/// The cards of the read notes in the window, on a layer over the rows, then their words.
pub(super) fn draw(
    ctx: &mut Ctx,
    rect: Rect,
    rows: Rows<'_>,
    (slots, drawn): (&[Slot], &[Rect]),
    scroll: f32,
) {
    if !slots.iter().any(floats) {
        return;
    }
    let block = Block::of(ctx, rows.gutters);
    ctx.layer();
    ctx.clipped(rect, |ctx| {
        for (from, to) in runs(slots) {
            pane(
                ctx,
                (drawn[from], drawn[to]),
                (slots[from], slots[to]),
                block,
            );
        }
    });
    code(ctx, rect, rows, scroll);
}

/// One card from its first row to its last: half a row inside where it opens or closes, run
/// past the window where the window cuts it.
fn pane(ctx: &mut Ctx, (first, last): (Rect, Rect), (opens, closes): (Slot, Slot), block: Block) {
    let across = card_across(ctx, first, block);
    let line = ctx.tokens.code_line;
    let top = match opens {
        Slot::Pad {
            room: Room::Over, ..
        } => first.h / 2.0,
        _ => -line,
    };
    let bottom = match closes {
        Slot::Pad {
            room: Room::Under, ..
        } => last.y + last.h / 2.0,
        _ => last.bottom() + line,
    };
    let card = groove_ui_kit::shape::band(across, top, bottom - first.y - top);
    glass_pane(ctx, card);
}

/// Each card's first and last row in the window, a card ending at the room under it.
fn runs(slots: &[Slot]) -> Vec<(usize, usize)> {
    let mut out: Vec<(usize, usize)> = Vec::new();
    let mut open = false;
    for (at, slot) in slots.iter().enumerate() {
        if !floats(slot) {
            open = false;
            continue;
        }
        match (open, out.last_mut()) {
            (true, Some(last)) => last.1 = at,
            _ => out.push((at, at)),
        }
        open = !matches!(
            slot,
            Slot::Pad {
                room: Room::Under,
                ..
            }
        );
    }
    out
}
