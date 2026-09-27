use groove_gfx::{Cell, CellGrid, CellSize, Color, Rect, WIDE_SPACER};
use groove_types::{Rgb, Screen, Selected};

use crate::base::ctx::{App, Ctx};

/// A terminal screen as a cell grid at `origin`, clipped to `rect`.
pub fn screen<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect, origin: (f32, f32), screen: &Screen) {
    let cell = ctx.cell;
    let ground = ctx.styles.deep();
    let grid = grid_of(screen, origin.0, origin.1, ctx.tokens.code, ground);
    let held = ctx.styles.held();
    ctx.clipped(rect, |ctx| {
        for one in &screen.selected {
            ctx.quad(selected(one, origin, cell), held);
        }
        ctx.grid(grid);
    });
}

/// The band under one row's selected columns.
fn selected(one: &Selected, origin: (f32, f32), cell: CellSize) -> Rect {
    Rect::new(
        origin.0 + one.from as f32 * cell.width,
        origin.1 + one.row as f32 * cell.height,
        one.to.saturating_sub(one.from) as f32 * cell.width,
        cell.height,
    )
}

/// The cells, the cursor drawn as the cell with its colours swapped.
pub fn grid_of(screen: &Screen, x: f32, y: f32, font_size: f32, ground: Color) -> CellGrid {
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
