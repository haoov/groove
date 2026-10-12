//! A selection of our own on a terminal's grid, the agent's or a shell's: press, drag, let go.

use groove_controllers::agent_service::Select;
use groove_controllers::{AppState, Command, agent, shell};

use crate::hit::{Hits, Target};
use crate::layout::{Layout, cell_at};
use crate::{Grid, Held, Ui};
use groove_ui_kit::base::ctx::Metrics;

/// A press begins one: the cell, the word on a second click, the line on a third.
pub(super) fn begun(ui: &mut Ui, app: &AppState, grid: Grid, cell: (usize, usize)) -> Vec<Command> {
    let kind = match ui.clicked.map(|one| one.count).unwrap_or(1) {
        1 => Select::Cells,
        2 => Select::Word,
        _ => Select::Line,
    };
    ui.held = Some(Held::Selecting(grid));
    said(app, grid, cell, kind, true)
}

/// The pointer moved while it is down: the selection it holds carried to its cell.
pub(super) fn carried(
    ui: &Ui,
    app: &AppState,
    point: (f32, f32),
    hits: &Hits,
    metrics: Metrics,
) -> Vec<Command> {
    let Some(Held::Selecting(grid)) = ui.held else {
        return Vec::new();
    };
    match cell(grid, point, ui, hits, metrics) {
        Some(cell) => said(app, grid, cell, Select::Cells, false),
        None => Vec::new(),
    }
}

/// What the grid's selection holds, to the clipboard.
pub(super) fn copied(app: &AppState, grid: Grid) -> Vec<Command> {
    let Some(session) = app.session.selected.clone() else {
        return Vec::new();
    };
    let command = match grid {
        Grid::Agent => Command::Agent(agent::Command::Copy { session }),
        Grid::Shell(id) => Command::Shell(shell::Command::Copy { session, id }),
    };
    vec![command]
}

/// The cell of the grid the point stands on.
fn cell(
    grid: Grid,
    point: (f32, f32),
    ui: &Ui,
    hits: &Hits,
    metrics: Metrics,
) -> Option<(usize, usize)> {
    match grid {
        Grid::Agent => Some(agent_cell(point, ui, metrics)),
        Grid::Shell(id) => shell_cell(id, point, hits, metrics),
    }
}

/// The cell of the agent's own grid the point stands on.
pub(super) fn agent_cell(point: (f32, f32), ui: &Ui, metrics: Metrics) -> (usize, usize) {
    let tokens = metrics.tokens();
    let origin = Layout::of(metrics, ui).agent_origin(&tokens);
    cell_at(origin, metrics.cell, point)
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
        Grid::Agent => Command::Agent(agent::Command::Select {
            session,
            col,
            row,
            kind,
            from,
        }),
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
