//! What a row offers the pointer: a file to fold, a gap to open.

use groove_gfx::Rect;
use groove_types::{DiffView, RowKind};

use super::row::Side;

use groove_controllers::workspace::Way;

use super::notes::Slot;
use super::row::Drawn;
use crate::components::{Gutters, head_mark, head_of};
use crate::ctx::Ctx;
use crate::hit::Target;
use crate::views::session::Face;
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;

/// What a head row offers (fold, mark read) and what a gap offers (its lines, from either end).
pub(super) fn marks(
    ctx: &mut Ctx,
    drawn: &[Rect],
    slots: &[Slot],
    rows: &[Drawn],
    first: usize,
    (gutters, view, side, rect): (Gutters, Face, Side, Rect),
) {
    ctx.clipped(rect, |ctx| {
        for (line, slot) in drawn.iter().zip(slots) {
            marked(ctx, *line, slot, rows, first, (gutters, view, side));
        }
    });
}

/// One row's marks: a gap's arrows, or a head's fold and read box.
fn marked(
    ctx: &mut Ctx,
    line: Rect,
    slot: &Slot,
    rows: &[Drawn],
    first: usize,
    numbers: (Gutters, Face, Side),
) {
    let Slot::Code(at) = slot else {
        return;
    };
    let Some(row) = rows.get(at - first) else {
        return;
    };
    if let RowKind::Gap(_) = row.kind {
        gap_marks(ctx, line, *at, numbers.0, numbers.1, numbers.2);
        return;
    }
    let split = numbers.1 == Face::Stream(DiffView::Split);
    let Some(path) = row.file.as_ref().filter(|_| row.head && !split) else {
        return;
    };
    head_hits(ctx, head_of(line, row.part), path);
}

/// A head folds its file on a click, and its box marks the file read.
pub(super) fn head_hits(ctx: &mut Ctx, whole: Rect, path: &str) {
    ctx.hit(whole, Target::Head(path.to_string()));
    ctx.hit(head_mark(ctx, whole), Target::Read(path.to_string()));
}

/// The arrows a gap wears: down where the old line stands, up where the new one does.
fn gap_marks(ctx: &mut Ctx, line: Rect, row: usize, gutters: Gutters, view: Face, side: Side) {
    ctx.hit(line, Target::Gap { row, way: Way::All });
    for (at, way) in ways(ctx, line, gutters, view, side) {
        let size = ctx.tokens.icon;
        let turn = match way {
            Way::Up => Mark::UPWARDS,
            _ => 0,
        };
        let mark = Rect::new(
            at.x + (at.w - size) / 2.0,
            at.y + (at.h - size) / 2.0,
            size,
            size,
        );
        groove_ui_kit::widgets::turned(ctx, mark, Mark::Down, turn, Role::Ghost);
        ctx.hit(at, Target::Gap { row, way });
    }
}

/// Where each arrow stands: one a gutter cell in inline, one a side in split.
fn ways(ctx: &mut Ctx, line: Rect, gutters: Gutters, view: Face, side: Side) -> Vec<(Rect, Way)> {
    let first = cell(ctx, line, gutters, 0);
    match (view, side) {
        (Face::Stream(DiffView::Inline), _) => {
            vec![(first, Way::Down), (cell(ctx, line, gutters, 1), Way::Up)]
        }
        (Face::Stream(DiffView::Split), Side::Old) => vec![(first, Way::Down)],
        (Face::Stream(DiffView::Split), Side::New) => vec![(first, Way::Up)],
        _ => {
            let half = Rect::new(first.x, first.y, first.w / 2.0, first.h);
            let other = Rect::new(half.right(), first.y, first.w / 2.0, first.h);
            vec![(half, Way::Down), (other, Way::Up)]
        }
    }
}

/// The rect one gutter cell of this row stands in.
fn cell(ctx: &mut Ctx, line: Rect, gutters: Gutters, at: usize) -> Rect {
    crate::components::Block::of(ctx, gutters).cell(ctx, line, at)
}
