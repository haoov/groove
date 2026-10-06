//! Rows of cells under a header of columns. Only the rows on screen are built.

mod act;
mod cell;

pub use act::Act;
pub use cell::Cell;

use groove_gfx::Rect;

use crate::base::ctx::{App, Ctx};
use crate::base::ground::Ground;
use crate::base::mark::Mark;
use crate::base::style::Role;
use crate::shape::{hairline, hoverable, ruled};

/// How wide a column is.
#[derive(Debug, Clone, Copy)]
pub enum Width<'a> {
    /// A share of what the fitted columns leave.
    Fill,
    /// As wide as this sample or the label, whichever is wider.
    Fit(&'a str),
}

pub struct Column<'a, T> {
    pub label: &'a str,
    pub width: Width<'a>,
    /// Its text stands against the right edge.
    pub end: bool,
    /// What a click on the label asks for.
    pub sort: Option<T>,
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

/// One row on screen: a cell a column, its second line, and what a click on it means.
pub struct Shown<'a, T> {
    pub cells: Vec<Cell<'a>>,
    /// Laid one after the other from where the first cell's text starts.
    pub under: Vec<Cell<'a>>,
    pub target: Option<T>,
    /// A button or a toggle in place of a cell's text, by column.
    pub acts: Vec<Option<Act<'a, T>>>,
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
    /// The header, then the rows `shown` builds for the indices on screen; returns the extent.
    pub fn draw<'r, A: App<Target = T>>(
        &self,
        ctx: &mut Ctx<'_, A>,
        rect: Rect,
        mut shown: impl FnMut(usize) -> Shown<'r, T>,
    ) -> f32 {
        let mut body = rect;
        let spans = self.spans(ctx, rect);
        if self.sorted.is_some() {
            let header = body.take_top(ctx.tokens.row);
            self.header(ctx, header, &spans);
        }
        let height = self.rows.first + self.rows.under;
        let extent = (self.count as f32 * height - body.h).max(0.0);
        let offset = self.offset.min(extent);
        let (first, last) = visible(offset, body.h, height, self.count);
        ctx.clipped(body, |ctx| {
            for at in first..last {
                let y = body.y - offset + at as f32 * height;
                let line = Rect::new(body.x, y, body.w, height);
                let selected = self.selected == Some(at);
                self.one(ctx, line, &spans, shown(at), selected);
            }
        });
        extent
    }

    /// Each column's left edge and width across `rect`.
    fn spans<A: App>(&self, ctx: &mut Ctx<'_, A>, rect: Rect) -> Vec<(f32, f32)> {
        let md = ctx.tokens.md;
        let (label, body) = (ctx.styles.small(Role::Faint), ctx.styles.small(Role::Text));
        let fitted: Vec<Option<f32>> = self
            .columns
            .iter()
            .map(|column| match column.width {
                Width::Fill => None,
                Width::Fit(sample) => {
                    let wide = ctx.measure(sample, &body);
                    Some(wide.max(ctx.measure(column.label, &label)) + ctx.tokens.icon)
                }
            })
            .collect();
        let gaps = md * (self.columns.len() + 1) as f32;
        let taken: f32 = fitted.iter().flatten().sum();
        let fills = fitted.iter().filter(|one| one.is_none()).count().max(1);
        let share = ((rect.w - gaps - taken) / fills as f32).max(0.0);
        let mut x = rect.x + md;
        fitted
            .into_iter()
            .map(|wide| {
                let w = wide.unwrap_or(share);
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
        }
    }

    fn one<A: App<Target = T>>(
        &self,
        ctx: &mut Ctx<'_, A>,
        line: Rect,
        spans: &[(f32, f32)],
        shown: Shown<'_, T>,
        selected: bool,
    ) {
        if selected {
            crate::shape::ground(ctx, line, Ground::Raised);
            ruled(ctx, line, ctx.styles.chosen());
        }
        if let Some(target) = shown.target {
            hoverable(ctx, line, target);
        }
        let mut rest = line;
        let first = rest.take_top(self.rows.first);
        let mut start = first.x + ctx.tokens.md;
        let mut acts = shown.acts.into_iter();
        for (at, (cell, (x, w))) in shown.cells.into_iter().zip(spans).enumerate() {
            let rect = Rect::new(*x, first.y, *w, first.h);
            if let Some(act) = acts.next().flatten() {
                act.draw(ctx, rect);
                continue;
            }
            let (text, _) = cell.draw(ctx, rect, self.columns[at].end);
            if at == 0 {
                start = text;
            }
        }
        under(ctx, rest.take_top(self.rows.under), start, shown.under);
        if self.rows.ruled && !selected {
            hairline(ctx, line, ctx.styles.line());
        }
    }
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

/// The first row on screen and the one past the last.
fn visible(offset: f32, room: f32, height: f32, count: usize) -> (usize, usize) {
    if height <= 0.0 {
        return (0, 0);
    }
    let first = (offset / height).floor() as usize;
    let last = ((offset + room) / height).ceil() as usize + 1;
    (first.min(count), last.min(count))
}
