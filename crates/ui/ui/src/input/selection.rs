//! A selection of our own on a shell's grid: press, drag, let go.

use groove_controllers::agent_service::Select;
use groove_controllers::{AppState, Command, shell};

use crate::hit::{Hits, Target};
use crate::layout::cell_at;
use crate::{Grid, Held, Ui};
use groove_ui_kit::base::ctx::Metrics;

/// A press: to the program that reads the mouse unless shift is held, else a selection begins.
pub(super) fn pressed(
    ui: &mut Ui,
    app: &AppState,
    grid: Grid,
    cell: (usize, usize),
) -> Vec<Command> {
    if reads_mouse(app, grid) && !ui.agent.bypassed {
        ui.held = Some(Held::Clicking(grid));
        return sent(app, grid, cell, Some(true));
    }
    let kind = match ui.clicked.map(|one| one.count).unwrap_or(1) {
        1 => Select::Cells,
        2 => Select::Word,
        _ => Select::Line,
    };
    ui.held = Some(Held::Selecting(grid));
    said(app, grid, cell, kind, true)
}

/// The pointer moved while it is down: the drag sent on, or the selection carried.
pub(super) fn carried(
    ui: &Ui,
    app: &AppState,
    point: (f32, f32),
    hits: &Hits,
    metrics: Metrics,
) -> Vec<Command> {
    let (grid, sending) = match ui.held {
        Some(Held::Selecting(grid)) => (grid, false),
        Some(Held::Clicking(grid)) => (grid, true),
        _ => return Vec::new(),
    };
    match (cell(grid, point, hits, metrics), sending) {
        (Some(cell), true) => sent(app, grid, cell, None),
        (Some(cell), false) => said(app, grid, cell, Select::Cells, false),
        (None, _) => Vec::new(),
    }
}

/// The button let go where the pointer stands, for the program that was sent the press.
pub(super) fn let_go(
    ui: &Ui,
    app: &AppState,
    grid: Grid,
    hits: &Hits,
    metrics: Metrics,
) -> Vec<Command> {
    match cell(grid, ui.at, hits, metrics) {
        Some(cell) => sent(app, grid, cell, Some(false)),
        None => Vec::new(),
    }
}

/// Whether the program on the grid reads the mouse itself.
fn reads_mouse(app: &AppState, grid: Grid) -> bool {
    let Some(session) = app.session.selected.as_ref() else {
        return false;
    };
    let terminal = match grid {
        Grid::Shell(id) => app.shell.terminal(session, id),
    };
    terminal.is_some_and(|one| one.reads_mouse())
}

/// The left button to the program: pressed, let go, or `None` for a drag.
fn sent(
    app: &AppState,
    grid: Grid,
    (col, row): (usize, usize),
    down: Option<bool>,
) -> Vec<Command> {
    let Some(session) = app.session.selected.clone() else {
        return Vec::new();
    };
    let command = match (grid, down) {
        (Grid::Shell(id), Some(down)) => Command::Shell(shell::Command::Click {
            session,
            id,
            col,
            row,
            down,
        }),
        (Grid::Shell(id), None) => Command::Shell(shell::Command::Drag {
            session,
            id,
            col,
            row,
        }),
    };
    vec![command]
}

/// What the grid's selection holds, to the clipboard.
pub(super) fn copied(app: &AppState, grid: Grid) -> Vec<Command> {
    let Some(session) = app.session.selected.clone() else {
        return Vec::new();
    };
    let command = match grid {
        Grid::Shell(id) => Command::Shell(shell::Command::Copy { session, id }),
    };
    vec![command]
}

/// The cell of the grid the point stands on.
fn cell(grid: Grid, point: (f32, f32), hits: &Hits, metrics: Metrics) -> Option<(usize, usize)> {
    match grid {
        Grid::Shell(id) => shell_cell(id, point, hits, metrics),
    }
}

/// The cell of one shell's grid the point stands on, while that grid is drawn.
pub(super) fn shell_cell(
    id: u64,
    point: (f32, f32),
    hits: &Hits,
    metrics: Metrics,
) -> Option<(usize, usize)> {
    let pane = hits.rect_of(&Target::Shell(id))?;
    let tokens = metrics.tokens();
    Some(cell_at(
        (pane.x + tokens.sm, pane.y + tokens.sm),
        metrics.cell,
        point,
    ))
}

fn said(
    app: &AppState,
    grid: Grid,
    (col, row): (usize, usize),
    kind: Select,
    from: bool,
) -> Vec<Command> {
    let Some(session) = app.session.selected.clone() else {
        return Vec::new();
    };
    let command = match grid {
        Grid::Shell(id) => Command::Shell(shell::Command::Select {
            session,
            id,
            col,
            row,
            kind,
            from,
        }),
    };
    vec![command]
}
