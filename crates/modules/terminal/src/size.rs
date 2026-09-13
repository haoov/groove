use std::sync::atomic::{AtomicU16, Ordering};

use alacritty_terminal::event::WindowSize;
use alacritty_terminal::grid::Dimensions;

/// The grid size, readable from the reader thread's listener.
pub(crate) struct Size {
    cols: AtomicU16,
    rows: AtomicU16,
}

/// A size at one instant, the shape `Term` reads.
#[derive(Clone, Copy)]
pub(crate) struct Dims {
    pub cols: usize,
    pub rows: usize,
}

impl Size {
    pub fn new(cols: u16, rows: u16) -> Self {
        Self {
            cols: AtomicU16::new(cols.max(1)),
            rows: AtomicU16::new(rows.max(1)),
        }
    }

    pub fn get(&self) -> (u16, u16) {
        (
            self.cols.load(Ordering::Relaxed),
            self.rows.load(Ordering::Relaxed),
        )
    }

    pub fn set(&self, cols: u16, rows: u16) {
        self.cols.store(cols.max(1), Ordering::Relaxed);
        self.rows.store(rows.max(1), Ordering::Relaxed);
    }

    pub fn dims(&self) -> Dims {
        let (cols, rows) = self.get();
        Dims {
            cols: usize::from(cols),
            rows: usize::from(rows),
        }
    }

    pub fn window(&self) -> WindowSize {
        let (cols, rows) = self.get();
        WindowSize {
            num_lines: rows,
            num_cols: cols,
            cell_width: 1,
            cell_height: 1,
        }
    }
}

impl Dimensions for Dims {
    fn total_lines(&self) -> usize {
        self.rows
    }

    fn screen_lines(&self) -> usize {
        self.rows
    }

    fn columns(&self) -> usize {
        self.cols
    }
}
