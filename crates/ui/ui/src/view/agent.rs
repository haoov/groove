use groove_controllers::AppState;
use groove_gfx::{Cell, CellGrid, Color, Frame, Rect, Theme, WIDE_SPACER, Weight};
use groove_types::{AgentStatus, Rgb, Screen};

use crate::layout::Layout;
use crate::view::sans;
use crate::widget::row;

/// The agent pane: the terminal's screen as a cell grid, the cursor as a swapped cell.
pub fn draw(frame: &mut Frame, app: &AppState, theme: &Theme, layout: &Layout) {
    let Some(open) = app.session.selected() else {
        return;
    };
    let p = &theme.palette;
    frame.quad(layout.agent, p.base);
    frame.quad(
        Rect::new(
            layout.agent.right() - 1.0,
            layout.agent.y,
            1.0,
            layout.agent.h,
        ),
        p.surface0,
    );

    let Some(agent) = app.agent.agent(&open.session.id) else {
        return note(frame, theme, layout, "starting the agent…");
    };
    match (&agent.terminal, &agent.activity.status) {
        (None, AgentStatus::Error { message }) => note(frame, theme, layout, message),
        (None, _) => note(frame, theme, layout, "starting the agent…"),
        (Some(terminal), _) => {
            let (x, y) = layout.agent_origin();
            let grid = grid_of(&terminal.screen(), x, y, layout.px(theme.mono), p.base);
            frame.clipped(layout.agent, |f| f.grid(grid));
        }
    }
}

fn note(frame: &mut Frame, theme: &Theme, layout: &Layout, text: &str) {
    let rect = Rect::new(
        layout.agent.x,
        layout.agent.y + layout.px(8.0),
        layout.agent.w,
        layout.row,
    );
    row(
        frame,
        rect,
        layout.px(12.0),
        text,
        sans(layout, theme.text, Weight::Regular, theme.palette.subtext0),
    );
}

/// A `Screen` as the renderer's grid; the cursor cell swaps its colours.
pub(crate) fn grid_of(screen: &Screen, x: f32, y: f32, font_size: f32, pane_bg: Color) -> CellGrid {
    let mut grid = CellGrid::new(x, y, screen.cols, screen.rows, font_size);
    for (i, sc) in screen.cells.iter().enumerate() {
        grid.cells[i] = Cell {
            ch: if sc.spacer { WIDE_SPACER } else { sc.ch },
            fg: color(sc.fg),
            bg: sc.bg.map(color).unwrap_or(Color::TRANSPARENT),
            bold: sc.bold,
        };
    }
    if let Some((col, row)) = screen
        .cursor
        .filter(|(c, r)| *c < screen.cols && *r < screen.rows)
    {
        let under = *grid.cell(col, row);
        let bg = if under.bg.is_transparent() {
            pane_bg
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
