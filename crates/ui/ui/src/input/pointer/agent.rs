//! What a click on the agent's own row does: its skills, or its reload.

use groove_controllers::{AppState, Command};
use groove_types::SessionId;

use crate::hit::{Hits, Target};
use crate::input::selection::{agent_cell, pressed as pressed_on};
use crate::{Corner, Grid, Menu, Of, Offer, Ui};
use groove_ui_kit::base::ctx::Metrics;

/// The agent's own targets; anything else is not its to answer.
pub(super) fn acted(
    target: &Target,
    point: (f32, f32),
    ui: &mut Ui,
    app: &AppState,
    hits: &Hits,
    metrics: Metrics,
) -> Option<Vec<Command>> {
    match target {
        Target::Skills(session) => Some(menu(ui, app, hits, session.clone())),
        Target::Agent => Some(pressed(ui, app, point, metrics)),
        _ => None,
    }
}

/// A press on the screen, through the grid's own rule.
fn pressed(ui: &mut Ui, app: &AppState, point: (f32, f32), metrics: Metrics) -> Vec<Command> {
    let cell = agent_cell(point, ui, metrics);
    pressed_on(ui, app, Grid::Agent, cell)
}

/// What the agent can be sent, under the word that opened it.
fn menu(ui: &mut Ui, app: &AppState, hits: &Hits, session: SessionId) -> Vec<Command> {
    let Some(open) = app.session.get(&session) else {
        return Vec::new();
    };
    let sources = groove_controllers::task_service::source_ids(app.config.config.as_ref());
    let offered: Vec<Offer> = app
        .skill
        .offers(&open.session.kind, &sources)
        .into_iter()
        .map(|one| Offer {
            id: one.id,
            args: one.args,
            label: one.label,
        })
        .collect();
    if offered.is_empty() {
        return Vec::new();
    }
    let at = hits
        .rect_of(&Target::Skills(session.clone()))
        .map(|word| (word.x, word.y))
        .unwrap_or_default();
    ui.overlay = Some(crate::Overlay::Menu(Menu {
        at,
        corner: Corner::BottomLeft,
        of: Of::Skills { session, offered },
    }));
    Vec::new()
}
