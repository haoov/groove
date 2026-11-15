use groove_gfx::{Cell, CellGrid, CellSize, Color, Rect, WIDE_SPACER};
use groove_types::{Rgb, Screen, Selected};

use crate::base::ctx::{App, Ctx};

/// A terminal screen at `origin`, clipped to `rect`; its cursor solid only while `focused`.
pub fn screen<A: App>(
    ctx: &mut Ctx<'_, A>,
    rect: Rect,
    origin: (f32, f32),
    (screen, focused): (&Screen, bool),
) {
    drawn(ctx, rect, origin, (screen, focused.then_some(None)));
}

/// The same, its cursor while `focused` a solid block in the accent: the agent's screen, which
/// has no edge for a ring and shows the keys by its caret alone.
pub fn accent_screen<A: App>(
    ctx: &mut Ctx<'_, A>,
    rect: Rect,
    origin: (f32, f32),
    (screen, focused): (&Screen, bool),
) {
    let accent = ctx.styles.color(crate::base::style::Role::Accent);
    drawn(ctx, rect, origin, (screen, focused.then_some(Some(accent))));
}

/// The screen; `solid` holds the focused cursor's ground, `None` inside it for its own colours swapped.
fn drawn<A: App>(
    ctx: &mut Ctx<'_, A>,
    rect: Rect,
    origin: (f32, f32),
    (screen, solid_in): (&Screen, Option<Option<Color>>),
) {
    let cell = ctx.cell;
    let ground = ctx.styles.deep();
    let mut grid = cells_of(screen, origin.0, origin.1, ctx.terminal);
    let cursor = cursor_of(screen);
    let focused = solid_in.is_some();
    match (solid_in, cursor) {
        (Some(None), Some(at)) => solid(&mut grid, at, ground),
        (Some(Some(accent)), Some(at)) => lit(&mut grid, at, (accent, ground)),
        _ => {}
    }
    let held = ctx.styles.held();
    ctx.clipped(rect, |ctx| {
        for one in &screen.selected {
            ctx.quad(selected(one, origin, cell), held);
        }
        ctx.grid(grid);
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

/// The cell under the cursor on `accent`, its glyph in `ground`.
fn lit(grid: &mut CellGrid, (col, row): (usize, usize), (accent, ground): (Color, Color)) {
    let under = *grid.cell(col, row);
    grid.set(
        col,
        row,
        Cell {
            fg: ground,
            bg: accent,
            ..under
        },
    );
}

fn color(rgb: Rgb) -> Color {
    Color::rgb(rgb.r, rgb.g, rgb.b)
}
