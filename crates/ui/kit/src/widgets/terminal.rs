use groove_gfx::{Cell, CellGrid, CellSize, Color, Rect, WIDE_SPACER};
use groove_types::{Rgb, Screen, Selected};

use crate::base::ctx::{App, Ctx};
use crate::base::style::Role;

/// A terminal screen at `origin`, clipped to `rect`; its cursor solid only while `focused`.
pub fn screen<A: App>(
    ctx: &mut Ctx<'_, A>,
    rect: Rect,
    origin: (f32, f32),
    (screen, focused): (&Screen, bool),
) {
    let cell = ctx.cell;
    let ground = ctx.styles.deep();
    let mut grid = cells_of(screen, origin.0, origin.1, ctx.terminal);
    let mut cursor = cursor_of(screen);
    let mut composed = None;
    if focused && let (Some(at), Some(text)) = (cursor, ctx.preedit()) {
        let (end, columns) = compose(&mut grid, at, text, ctx.styles.color(Role::Text), ground);
        composed = Some((at, columns));
        cursor = Some(end);
    }
    if focused && let Some(at) = cursor {
        solid(&mut grid, at, ground);
    }
    let held = ctx.styles.held();
    ctx.clipped(rect, |ctx| {
        for one in &screen.selected {
            ctx.quad(selected(one, origin, cell), held);
        }
        ctx.grid(grid);
        if let Some(((col, row), columns)) = composed {
            let x = origin.0 + col as f32 * cell.width;
            let y = origin.1 + (row + 1) as f32 * cell.height - ctx.tokens.hairline;
            let rule = Rect::new(x, y, columns as f32 * cell.width, ctx.tokens.hairline);
            ctx.quad(rule, ctx.styles.color(Role::Text));
        }
        if focused && let Some((col, row)) = cursor {
            let x = origin.0 + col as f32 * cell.width;
            let y = origin.1 + row as f32 * cell.height;
            ctx.app.caret(Rect::new(x, y, cell.width, cell.height));
        }
        if !focused && let Some((col, row)) = cursor {
            let x = origin.0 + col as f32 * cell.width;
            let y = origin.1 + row as f32 * cell.height;
            let caret = ctx.styles.caret();
            ctx.border(Rect::new(x, y, cell.width, cell.height), caret);
        }
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
    let mut grid = cells_of(screen, x, y, font_size);
    if let Some(at) = cursor_of(screen) {
        solid(&mut grid, at, ground);
    }
    grid
}

fn cells_of(screen: &Screen, x: f32, y: f32, font_size: f32) -> CellGrid {
    let mut grid = CellGrid::new(x, y, screen.cols, screen.rows, font_size);
    for (i, cell) in screen.cells.iter().enumerate() {
        grid.cells[i] = Cell {
            ch: if cell.spacer { WIDE_SPACER } else { cell.ch },
            fg: color(cell.fg),
            bg: cell.bg.map(color).unwrap_or(Color::TRANSPARENT),
            bold: cell.bold,
        };
    }
    grid
}

fn cursor_of(screen: &Screen) -> Option<(usize, usize)> {
    screen
        .cursor
        .filter(|(col, row)| *col < screen.cols && *row < screen.rows)
}

/// The cell under the cursor with its colours swapped.
fn solid(grid: &mut CellGrid, (col, row): (usize, usize), ground: Color) {
    let under = *grid.cell(col, row);
    let bg = match under.bg.is_transparent() {
        true => ground,
        false => under.bg,
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

fn color(rgb: Rgb) -> Color {
    Color::rgb(rgb.r, rgb.g, rgb.b)
}

/// The preedit over the cells from the cursor, on the line's end; returns where the cursor
/// stands after it and how many columns it took.
fn compose(
    grid: &mut CellGrid,
    (mut col, row): (usize, usize),
    text: &str,
    fg: Color,
    ground: Color,
) -> ((usize, usize), usize) {
    let start = col;
    let cols = grid.cols;
    for ch in text.chars() {
        let wide = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if col + wide > cols {
            break;
        }
        if wide == 0 {
            continue;
        }
        let one = Cell {
            ch,
            fg,
            bg: ground,
            bold: false,
        };
        grid.set(col, row, one);
        if wide == 2 {
            let spacer = Cell {
                ch: WIDE_SPACER,
                ..one
            };
            grid.set(col + 1, row, spacer);
        }
        col += wide;
    }
    ((col.min(cols - 1), row), col - start)
}
