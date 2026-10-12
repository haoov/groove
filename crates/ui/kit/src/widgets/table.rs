//! Rows of cells under a header of columns. Only the rows on screen are built.

mod act;
mod cell;
mod laid;
mod shown;

pub use act::Act;
pub use cell::Cell;
pub use laid::Laid;
pub use shown::{Band, Line, Shown, Slot};

use groove_gfx::Rect;

use crate::base::ctx::{App, Ctx};
use crate::base::mark::Mark;
use crate::base::style::Role;
use crate::shape::{hairline, hoverable};
use laid::visible;

/// How wide a column is.
#[derive(Debug, Clone, Copy)]
pub enum Width<'a> {
    /// A share of what the fitted columns leave.
    Fill,
    /// As wide as this sample in the small size or the label, whichever is wider.
    Fit(&'a str),
    /// As wide as this cell draws, no slack, or the label, whichever is wider.
    FitCell(&'a Cell<'a>),
    /// This many pixels, as the user dragged it.
    Fixed(f32),
}

pub struct Column<'a, T> {
    pub label: &'a str,
    pub width: Width<'a>,
    /// Its text stands against the right edge.
    pub end: bool,
    /// What a click on the label asks for.
    pub sort: Option<T>,
    /// What a press on the boundary after it holds, to drag its width.
    pub edge: Option<T>,
}

/// Which column orders the rows, and which way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Sorted {
    pub column: usize,
    pub descending: bool,
}

impl Sorted {
    /// A header whose rows keep their own order.
    pub const NONE: Sorted = Sorted {
        column: usize::MAX,
        descending: false,
    };
}

/// How tall a row stands, and whether a rule closes it.
#[derive(Debug, Clone, Copy)]
pub struct Rows {
    pub first: f32,
    /// The second line's height; nothing when the rows have none.
    pub under: f32,
    pub ruled: bool,
}

pub struct Table<'a, T> {
    pub columns: &'a [Column<'a, T>],
    pub rows: Rows,
    pub count: usize,
    /// How far the rows are scrolled, in pixels.
    pub offset: f32,
    pub selected: Option<usize>,
    /// The header row, which sorts; `None` draws the rows alone.
    pub sorted: Option<Sorted>,
}

impl<T: Clone + PartialEq> Table<'_, T> {
    /// The header, then the rows `shown` builds for the indices on screen.
    pub fn draw<'r, A: App<Target = T>>(
        &self,
        ctx: &mut Ctx<'_, A>,
        rect: Rect,
        mut shown: impl FnMut(usize) -> Shown<'r, T>,
    ) -> Laid {
        let mut body = rect;
        let spans = self.spans(ctx, rect);
        if self.sorted.is_some() {
            let header = body.take_top(ctx.tokens.row);
            self.header(ctx, header, &spans);
        }
        let height = self.rows.first + self.rows.under;
        let extent = (self.count as f32 * height - body.h).max(0.0);
        let offset = self.offset.min(extent);
        let laid = Laid {
            extent,
            spans,
            body,
            offset,
            height,
            first: self.rows.first,
        };
        let (first, last) = visible(offset, body.h, height, self.count);
        ctx.clipped(body, |ctx| {
            for at in first..last {
                let selected = self.selected == Some(at);
                self.line(ctx, laid.line(at), &laid.spans, shown(at), selected);
            }
        });
        laid
    }

    /// One row alone at the top of `rect`, under the header when there is one.
    pub fn one<A: App<Target = T>>(
        &self,
        ctx: &mut Ctx<'_, A>,
        rect: Rect,
        shown: Shown<'_, T>,
    ) -> Laid {
        let single = Table {
            count: 1,
            offset: 0.0,
            ..*self
        };
        let mut shown = Some(shown);
        single.draw(ctx, rect, |_| {
            shown
                .take()
                .unwrap_or_else(|| Shown::row(Vec::<Cell>::new()))
        })
    }

    /// Each column's left edge and width across `rect`.
    pub fn spans<A: App>(&self, ctx: &mut Ctx<'_, A>, rect: Rect) -> Vec<(f32, f32)> {
        let md = ctx.tokens.md;
        let (label, body) = (ctx.styles.small(Role::Faint), ctx.styles.small(Role::Text));
        let fitted: Vec<Option<f32>> = self
            .columns
            .iter()
            .map(|column| {
                let (wide, slack) = match column.width {
                    Width::Fill => return None,
                    Width::Fixed(wide) => return Some(wide),
                    Width::Fit(sample) => (ctx.measure(sample, &body), ctx.tokens.icon),
                    Width::FitCell(sample) => (sample.wide(ctx), 0.0),
                };
                Some(wide.max(ctx.measure(column.label, &label)) + slack)
            })
            .collect();
        let room = (rect.w - md * (self.columns.len() + 1) as f32).max(0.0);
        let least = ctx.tokens.aside_near;
        let wanted: f32 = fitted.iter().map(|one| one.unwrap_or(least)).sum();
        let fills = fitted.iter().filter(|one| one.is_none()).count().max(1);
        let (squeeze, share) = match wanted > room {
            true => (room / wanted.max(1.0), 0.0),
            false => (1.0, (room - wanted) / fills as f32),
        };
        let mut x = rect.x + md;
        fitted
            .into_iter()
            .map(|wide| {
                let w = wide.unwrap_or(least + share) * squeeze;
                let span = (x, w);
                x += w + md;
                span
            })
            .collect()
    }

    fn header<A: App<Target = T>>(&self, ctx: &mut Ctx<'_, A>, line: Rect, spans: &[(f32, f32)]) {
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

    fn line<A: App<Target = T>>(
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

fn banded<A: App>(ctx: &mut Ctx<'_, A>, line: Rect, band: Band<'_, A::Target>, selected: bool) {
    crate::shape::ground(ctx, line, crate::base::ground::Ground::Band);
    if selected {
        crate::shape::chosen(ctx, line);
    }
    if let Some(target) = band.target {
        hoverable(ctx, line, target);
    }
    under(ctx, line, line.x + ctx.tokens.md, band.cells);
    hairline(ctx, line, ctx.styles.line());
}

/// The second line's cells one after the other from `x`, a small gap between them.
fn under<A: App>(ctx: &mut Ctx<'_, A>, line: Rect, x: f32, cells: Vec<Cell<'_>>) {
    if line.h <= 0.0 {
        return;
    }
    let right = line.right() - ctx.tokens.md;
    let mut at = x;
    for cell in cells {
        let room = Rect::new(at, line.y, (right - at).max(0.0), line.h);
        let (from, to) = cell.draw(ctx, room, false);
        if to > from {
            at = to + ctx.tokens.sm;
        }
    }
}

#[cfg(test)]
mod tests;
