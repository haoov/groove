//! What a row offers the pointer: a file to fold, a gap to open.

use groove_gfx::Rect;
use groove_types::{DiffView, RowKind};

use super::row::Side;

use groove_controllers::workspace::Way;

use super::notes::Slot;
use super::row::Drawn;
use crate::ctx::Ctx;
use crate::hit::Target;
use crate::mark::Mark;
use crate::style::Role;
use crate::widget::{Gutters, head_mark};

/// What a head row offers: the row itself folds, its box marks the file read. What a
/// gap offers: the lines it hides, from either end or whole.
pub(super) fn marks(
    ctx: &mut Ctx,
    drawn: &[Rect],
    slots: &[Slot],
    rows: &[Drawn],
    first: usize,
    numbers: (Gutters, DiffView, Side),
) {
    for (line, slot) in drawn.iter().zip(slots) {
        let Slot::Code(at) = slot else {
            continue;
        };
        let Some(row) = rows.get(at - first) else {
            continue;
        };
        if let RowKind::Gap(_) = row.kind {
            gap_marks(ctx, *line, *at, numbers.0, numbers.1, numbers.2);
            continue;
        }
        let Some(path) = row.file.as_ref().filter(|_| row.head) else {
            continue;
        };
        ctx.hit(*line, Target::Head(path.clone()));
        ctx.hit(head_mark(ctx, *line), Target::Read(path.clone()));
    }
}

/// The arrows a gap wears: down where the old line stands, up where the new one does.
fn gap_marks(ctx: &mut Ctx, line: Rect, row: usize, gutters: Gutters, view: DiffView, side: Side) {
    ctx.hit(line, Target::Gap { row, way: Way::All });
    let colour = ctx.styles.color(Role::Ghost);
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
        ctx.icon(mark, Mark::Down, turn, colour);
        ctx.hit(at, Target::Gap { row, way });
    }
}

/// Where each arrow stands: one a gutter cell in inline, one a side in split.
fn ways(
    ctx: &mut Ctx,
    line: Rect,
    gutters: Gutters,
    view: DiffView,
    side: Side,
) -> Vec<(Rect, Way)> {
    let first = cell(ctx, line, gutters, 0);
    match (view, side) {
        (DiffView::Inline, _) => vec![(first, Way::Down), (cell(ctx, line, gutters, 1), Way::Up)],
        (DiffView::Split, Side::Old) => vec![(first, Way::Down)],
        (DiffView::Split, Side::New) => vec![(first, Way::Up)],
        _ => {
            let half = Rect::new(first.x, first.y, first.w / 2.0, first.h);
            let other = Rect::new(half.right(), first.y, first.w / 2.0, first.h);
            vec![(half, Way::Down), (other, Way::Up)]
        }
    }
}

/// The rect one gutter cell of this row stands in.
fn cell(ctx: &mut Ctx, line: Rect, gutters: Gutters, at: usize) -> Rect {
    let small = ctx.tokens.sm;
    let style = ctx.styles.code(Role::Ghost);
    let width = ctx.measure(&"0".repeat(gutters.digits), &style);
    let x = line.x + small + at as f32 * (width + small);
    Rect::new(x, line.y, width, line.h)
}
