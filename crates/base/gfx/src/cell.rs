use crate::{Color, Rect};

/// The second cell of a wide character.
pub const WIDE_SPACER: char = '\0';

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Cell {
    pub ch: char,
    pub fg: Color,
    pub bg: Color,
    pub bold: bool,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            fg: Color::WHITE,
            bg: Color::TRANSPARENT,
            bold: false,
        }
    }
}

impl Cell {
    pub fn is_blank(&self) -> bool {
        self.ch == ' ' || self.ch == WIDE_SPACER
    }
}

/// A monospace grid: the terminal and the editor.
#[derive(Clone, PartialEq, Debug)]
pub struct CellGrid {
    pub x: f32,
    pub y: f32,
    pub cols: usize,
    pub rows: usize,
    pub font_size: f32,
    pub cells: Vec<Cell>,
    pub(crate) clip: Rect,
}

impl CellGrid {
    pub fn new(x: f32, y: f32, cols: usize, rows: usize, font_size: f32) -> Self {
        Self {
            x,
            y,
            cols,
            rows,
            font_size,
            cells: vec![Cell::default(); cols * rows],
            clip: Rect::default(),
        }
    }

    pub fn cell(&self, col: usize, row: usize) -> &Cell {
        &self.cells[row * self.cols + col]
    }

    pub fn set(&mut self, col: usize, row: usize, cell: Cell) {
        self.cells[row * self.cols + col] = cell;
    }

    /// Writes `text` from `col`, one char per cell, clipped at the row's end.
    pub fn write(&mut self, col: usize, row: usize, text: &str, fg: Color, bold: bool) {
        for (i, ch) in text.chars().enumerate() {
            let col = col + i;
            if col >= self.cols {
                break;
            }
            let bg = self.cell(col, row).bg;
            self.set(col, row, Cell { ch, fg, bg, bold });
        }
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (usize, usize, &Cell)> {
        self.cells
            .iter()
            .enumerate()
            .map(|(i, c)| (i % self.cols, i / self.cols, c))
    }
}
