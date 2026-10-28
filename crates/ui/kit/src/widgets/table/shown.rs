//! One row on screen: a group's band across the columns, or a row of slots under them.

use groove_gfx::{Edges, Rect};

use super::{Act, Cell};
use crate::base::ctx::{App, Ctx};
use crate::base::style::Role;
use crate::shape::hairline;

pub enum Shown<'a, T> {
    Band(Band<'a, T>),
    Row(Line<'a, T>),
}

/// A group's heading: its cells one after the other across every column, on the band's ground.
pub struct Band<'a, T> {
    pub cells: Vec<Cell<'a>>,
    /// Whether what it heads is open, when it folds.
    pub open: Option<bool>,
    pub target: Option<T>,
}

/// A slot a column, its second line, and what a click on it means.
pub struct Line<'a, T> {
    pub slots: Vec<Slot<'a, T>>,
    /// Laid one after the other from where the first cell's text starts.
    pub under: Vec<Cell<'a>>,
    pub target: Option<T>,
}

/// What a column holds in a row: a cell, or a button or a toggle in its place.
pub enum Slot<'a, T> {
    Cell(Cell<'a>),
    Act(Act<'a, T>),
}

impl<'a, T> From<Cell<'a>> for Slot<'a, T> {
    fn from(cell: Cell<'a>) -> Self {
        Slot::Cell(cell)
    }
}

impl<'a, T> From<Act<'a, T>> for Slot<'a, T> {
    fn from(act: Act<'a, T>) -> Self {
        Slot::Act(act)
    }
}

impl<'a, T> Shown<'a, T> {
    /// A row of `slots`, one a column from the first.
    pub fn row<S: Into<Slot<'a, T>>>(slots: Vec<S>) -> Self {
        Shown::Row(Line {
            slots: slots.into_iter().map(Into::into).collect(),
            under: Vec::new(),
            target: None,
        })
    }

    pub fn band(cells: Vec<Cell<'a>>) -> Self {
        Shown::Band(Band {
            cells,
            open: None,
            target: None,
        })
    }

    /// What a click on it means.
    pub fn target(mut self, target: impl Into<Option<T>>) -> Self {
        let target = target.into();
        match &mut self {
            Shown::Band(band) => band.target = target,
            Shown::Row(row) => row.target = target,
        }
        self
    }

    /// A caret before a band's cells; a row folds through `Cell::fold`.
    pub fn fold(mut self, open: bool) -> Self {
        if let Shown::Band(band) = &mut self {
            band.open = Some(open);
        }
        self
    }

    pub fn under(mut self, cells: Vec<Cell<'a>>) -> Self {
        match &mut self {
            Shown::Row(row) => row.under = cells,
            Shown::Band(_) => debug_assert!(cells.is_empty(), "a band has no second line"),
        }
        self
    }
}

pub(super) fn banded<A: App>(
    ctx: &mut Ctx<'_, A>,
    line: Rect,
    band: Band<'_, A::Target>,
    (selected, ruled): (bool, bool),
) {
    if ruled {
        crate::shape::ground(ctx, line, crate::base::ground::Ground::Band);
    }
    if selected {
        crate::widgets::glass_chosen(ctx, line);
    }
    if let Some(target) = band.target {
        crate::widgets::glass_hoverable(ctx, line, target);
    }
    let mut room = line.pad(Edges::across(ctx.tokens.md, 0.0));
    if let Some(open) = band.open {
        crate::widgets::fold(ctx, &mut room, open, Role::Ghost);
    }
    under(ctx, line, room.x, band.cells);
    if ruled {
        hairline(ctx, line, ctx.styles.line());
    }
}

/// The second line's cells one after the other from `x`, a small gap between them.
pub(super) fn under<A: App>(ctx: &mut Ctx<'_, A>, line: Rect, x: f32, cells: Vec<Cell<'_>>) {
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
