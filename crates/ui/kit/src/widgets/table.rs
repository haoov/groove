//! Rows of cells under a header of columns. Only the rows on screen are built.

mod act;
mod cell;
mod laid;
mod painted;
mod shown;

pub use act::Act;
pub use cell::Cell;
pub use laid::Laid;
pub use shown::{Band, Line, Shown, Slot};

use groove_gfx::Rect;

use crate::base::ctx::{App, Ctx};
use crate::base::style::Role;
use laid::visible;

/// How wide a column is.
#[derive(Debug, Clone, Copy)]
pub enum Width<'a> {
    /// A share of what the fitted columns leave.
    Fill,
    /// As wide as this sample in the small size or the label, whichever is wider.
    Fit(&'a str),
    /// As wide as this sample in the body size or the label, whichever is wider.
    FitBody(&'a str),
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

impl<'a, T> Column<'a, T> {
    /// A column that neither sorts nor drags, its text from the left.
    pub fn plain(label: &'a str, width: Width<'a>) -> Self {
        Self {
            label,
            width,
            end: false,
            sort: None,
            edge: None,
        }
    }
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
    /// Each row a rounded card on glass instead of a ruled line.
    pub carded: bool,
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
        let laid = self.laid(ctx, rect);
        if self.sorted.is_some() {
            let mut head = rect;
            let header = head.take_top(ctx.tokens.row);
            self.header(ctx, header, &laid.spans);
        }
        let body = laid.body;
        let (first, last) = visible(laid.offset, body.h, laid.height, self.count);
        ctx.clipped(body, |ctx| {
            for at in first..last {
                let selected = self.selected == Some(at);
                self.line(ctx, laid.line(at), &laid.spans, shown(at), selected);
            }
        });
        laid
    }

    /// Where the header and the rows would stand in `rect`, drawing nothing.
    pub fn laid<A: App>(&self, ctx: &mut Ctx<'_, A>, rect: Rect) -> Laid {
        let mut body = rect;
        let spans = self.spans(ctx, rect);
        if self.sorted.is_some() {
            body.take_top(ctx.tokens.row);
        }
        let inset = match self.rows.carded {
            true => crate::widgets::card_inset(&ctx.tokens),
            false => 0.0,
        };
        let height = self.rows.first + self.rows.under + inset * 2.0;
        let extent = (self.count as f32 * height - body.h).max(0.0);
        Laid {
            extent,
            spans,
            body,
            offset: self.offset.min(extent),
            height,
            first: self.rows.first,
            inset,
        }
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
        let label = ctx.styles.small(Role::Faint);
        let (small, body) = (ctx.styles.small(Role::Text), ctx.styles.body(Role::Text));
        let fitted: Vec<Option<f32>> = self
            .columns
            .iter()
            .map(|column| {
                let (wide, slack) = match column.width {
                    Width::Fill => return None,
                    Width::Fixed(wide) => return Some(wide),
                    Width::Fit(sample) => (ctx.measure(sample, &small), ctx.tokens.icon),
                    Width::FitBody(sample) => (ctx.measure(sample, &body), ctx.tokens.icon),
                    Width::FitCell(sample) => {
                        (sample.wide(ctx).max(cell::sort_room(ctx, column)), 0.0)
                    }
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
}

#[cfg(test)]
mod tests;
