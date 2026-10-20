//! A table's header over its columns, and one of its rows, as they are drawn.

use groove_gfx::Rect;

use super::shown::{banded, under};
use super::{Cell, Line, Shown, Slot, Table};
use crate::base::ctx::{App, Ctx};
use crate::base::mark::Mark;
use crate::base::style::Role;
use crate::shape::{hairline, hoverable};

impl<T: Clone + PartialEq> Table<'_, T> {
    pub(super) fn header<A: App<Target = T>>(
        &self,
        ctx: &mut Ctx<'_, A>,
        line: Rect,
        spans: &[(f32, f32)],
    ) {
        hairline(ctx, line, ctx.styles.line());
        for (at, column) in self.columns.iter().enumerate() {
            let (x, w) = spans[at];
            let rect = Rect::new(x, line.y, w, line.h);
            let sorted = self.sorted.filter(|sorted| sorted.column == at);
            let role = match sorted.is_some() {
                true => Role::Text,
                false => Role::Faint,
            };
            if let Some(target) = column.sort.clone() {
                ctx.hit(rect, target);
            }
            let mut cell = Cell::small(column.label, role);
            if let Some(sorted) = sorted {
                let turn = match sorted.descending {
                    true => 0,
                    false => Mark::UPWARDS,
                };
                cell = cell.mark(Mark::Down, turn, role);
            }
            cell.draw(ctx, rect, column.end);
            if let Some(target) = column.edge.clone() {
                let gap = Rect::new(x + w, line.y, ctx.tokens.md, line.h);
                ctx.hit(gap, target);
            }
        }
    }

    pub(super) fn line<A: App<Target = T>>(
        &self,
        ctx: &mut Ctx<'_, A>,
        line: Rect,
        spans: &[(f32, f32)],
        shown: Shown<'_, T>,
        selected: bool,
    ) {
        match shown {
            Shown::Band(band) => banded(ctx, line, band, selected),
            Shown::Row(row) => self.row(ctx, line, spans, row, selected),
        }
    }

    fn row<A: App<Target = T>>(
        &self,
        ctx: &mut Ctx<'_, A>,
        line: Rect,
        spans: &[(f32, f32)],
        row: Line<'_, T>,
        selected: bool,
    ) {
        debug_assert!(
            row.slots.len() <= spans.len(),
            "a slot past the last column"
        );
        if selected {
            crate::shape::chosen(ctx, line);
        }
        if let Some(target) = row.target {
            hoverable(ctx, line, target);
        }
        let mut rest = line;
        let first = rest.take_top(self.rows.first);
        let mut start = first.x + ctx.tokens.md;
        for (at, (slot, (x, w))) in row.slots.into_iter().zip(spans).enumerate() {
            let rect = Rect::new(*x, first.y, *w, first.h);
            match slot {
                Slot::Act(act) => act.draw(ctx, rect),
                Slot::Cell(cell) => {
                    let (text, _) = cell.draw(ctx, rect, self.columns[at].end);
                    if at == 0 {
                        start = text;
                    }
                }
            }
        }
        under(ctx, rest.take_top(self.rows.under), start, row.under);
        if self.rows.ruled && !selected {
            hairline(ctx, line, ctx.styles.line());
        }
    }
}
