//! One row on screen: a group's band across the columns, or a row of slots under them.

use super::{Act, Cell};

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
