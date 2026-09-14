use groove_gfx::{Cell, CellGrid, Color, Rect, WIDE_SPACER};
use groove_types::{Rgb, Screen};

use crate::ctx::Ctx;

/// A terminal screen as a cell grid at `origin`, clipped to `rect`.
pub fn screen(ctx: &mut Ctx, rect: Rect, origin: (f32, f32), screen: &Screen) {
    let ground = ctx.styles.ground();
    let grid = grid_of(screen, origin.0, origin.1, ctx.tokens.code, ground);
    ctx.clipped(rect, |ctx| ctx.grid(grid));
}

/// The cells, the cursor drawn as the cell with its colours swapped.
pub(crate) fn grid_of(screen: &Screen, x: f32, y: f32, font_size: f32, ground: Color) -> CellGrid {
    let mut grid = CellGrid::new(x, y, screen.cols, screen.rows, font_size);
    for (i, cell) in screen.cells.iter().enumerate() {
        grid.cells[i] = Cell {
            ch: if cell.spacer { WIDE_SPACER } else { cell.ch },
            fg: color(cell.fg),
            bg: cell.bg.map(color).unwrap_or(Color::TRANSPARENT),
            bold: cell.bold,
        };
    }
    let inside = screen
        .cursor
        .filter(|(col, row)| *col < screen.cols && *row < screen.rows);
    if let Some((col, row)) = inside {
        let under = *grid.cell(col, row);
        let bg = if under.bg.is_transparent() {
            ground
        } else {
            under.bg
        };
        grid.set(
            col,
            row,
            Cell {
                fg: bg,
                bg: under.fg,
                ..under
            },
        );
    }
    grid
}

fn color(rgb: Rgb) -> Color {
    Color::rgb(rgb.r, rgb.g, rgb.b)
}
