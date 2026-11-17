//! What a click on the agent's screen does.

use groove_controllers::{AppState, Command};

use crate::hit::Target;
use crate::input::selection::{agent_cell, pressed as pressed_on};
use crate::{Grid, Ui};
use groove_ui_kit::base::ctx::Metrics;

/// The agent's own targets; anything else is not its to answer.
pub(super) fn acted(
    target: &Target,
    point: (f32, f32),
    ui: &mut Ui,
    app: &AppState,
    metrics: Metrics,
) -> Option<Vec<Command>> {
    match target {
        Target::Agent => Some(pressed(ui, app, point, metrics)),
        _ => None,
    }
}

/// A press on the screen, through the grid's own rule.
fn pressed(ui: &mut Ui, app: &AppState, point: (f32, f32), metrics: Metrics) -> Vec<Command> {
    let cell = agent_cell(point, ui, metrics);
    pressed_on(ui, app, Grid::Agent, cell)
}
