//! What a click on the agent's own row does: its skills, or its reload.

use groove_controllers::{AppState, Command, agent};
use groove_types::SessionId;

use crate::hit::{Hits, Target};
use crate::input::selection::{agent_cell, begun};
use crate::layout::Layout;
use crate::{Corner, Grid, Held, Menu, Of, Offer, Ui};
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
        Target::Reload(session) => Some(reloaded(ui, session.clone(), metrics)),
        Target::AutoApprove(session) => Some(switched(app, session.clone())),
        Target::Agent => Some(pressed(ui, app, point, metrics)),
        _ => None,
    }
}

/// A press on the screen: the program that reads the mouse is sent it, shift aside.
fn pressed(ui: &mut Ui, app: &AppState, point: (f32, f32), metrics: Metrics) -> Vec<Command> {
    let Some(session) = app.session.selected.clone() else {
        return Vec::new();
    };
    let (col, row) = agent_cell(point, ui, metrics);
    if reads_mouse(app, &session) && !ui.agent.bypassed {
        ui.held = Some(Held::AgentClick);
        return vec![Command::Agent(agent::Command::Click {
            session,
            col,
            row,
            down: true,
        })];
    }
    begun(ui, app, Grid::Agent, (col, row))
}

/// The pointer moved while it is down, sent on to the program that was sent the press.
pub(super) fn dragged(
    ui: &Ui,
    app: &AppState,
    point: (f32, f32),
    metrics: Metrics,
) -> Vec<Command> {
    let Some(session) = app.session.selected.clone() else {
        return Vec::new();
    };
    let (col, row) = agent_cell(point, ui, metrics);
    vec![Command::Agent(agent::Command::Drag { session, col, row })]
}

/// The button let go where it stands, for the program that was sent the press.
pub(crate) fn released(ui: &Ui, app: &AppState, metrics: Metrics) -> Vec<Command> {
    let Some(session) = app.session.selected.clone() else {
        return Vec::new();
    };
    let (col, row) = agent_cell(ui.at, ui, metrics);
    vec![Command::Agent(agent::Command::Click {
        session,
        col,
        row,
        down: false,
    })]
}

/// Whether the program in the agent reads the mouse itself.
fn reads_mouse(app: &AppState, session: &SessionId) -> bool {
    app.agent
        .agent(session)
        .and_then(|one| one.terminal.as_ref())
        .is_some_and(|one| one.reads_mouse())
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

/// Every write of the session let through without asking, or asking again.
fn switched(app: &AppState, session: SessionId) -> Vec<Command> {
    let on = !app
        .session
        .get(&session)
        .is_some_and(|one| one.state.auto_approve);
    vec![Command::Agent(agent::Command::AutoApprove { session, on })]
}

/// The agent ended and started again, on the grid its pane holds now.
fn reloaded(ui: &Ui, session: SessionId, metrics: Metrics) -> Vec<Command> {
    let tokens = metrics.tokens();
    let (cols, rows) = Layout::of(metrics, ui).agent_grid(&tokens, metrics.cell);
    vec![Command::Agent(agent::Command::Reload {
        session,
        cols,
        rows,
    })]
}
