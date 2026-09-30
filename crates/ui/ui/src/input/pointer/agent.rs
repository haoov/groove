//! What a click on the agent's own row does: its skills, or its reload.

use groove_controllers::agent_service::Select;
use groove_controllers::{AppState, Command, agent};
use groove_types::{ProviderId, SessionId};

use crate::hit::{Hits, Target};
use crate::layout::Layout;
use crate::{Corner, Held, Menu, Of, Offer, Ui};
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
    let (col, row) = cell(point, ui, metrics);
    if reads_mouse(app, &session) && !ui.agent.bypassed {
        ui.held = Some(Held::AgentClick);
        return vec![Command::Agent(agent::Command::Click {
            session,
            col,
            row,
            down: true,
        })];
    }
    let kind = match ui.clicked.map(|one| one.count).unwrap_or(1) {
        1 => Select::Cells,
        2 => Select::Word,
        _ => Select::Line,
    };
    ui.held = Some(Held::AgentText);
    vec![Command::Agent(agent::Command::Select {
        session,
        col,
        row,
        kind,
        from: true,
    })]
}

/// The pointer moved while it is down: the drag sent on, or the selection carried.
pub(super) fn dragged(
    ui: &Ui,
    app: &AppState,
    point: (f32, f32),
    metrics: Metrics,
) -> Vec<Command> {
    let Some(session) = app.session.selected.clone() else {
        return Vec::new();
    };
    let (col, row) = cell(point, ui, metrics);
    match matches!(ui.held, Some(Held::AgentClick)) {
        true => vec![Command::Agent(agent::Command::Drag { session, col, row })],
        false => vec![Command::Agent(agent::Command::Select {
            session,
            col,
            row,
            kind: Select::Cells,
            from: false,
        })],
    }
}

/// The button let go where it stands, for the program that was sent the press.
pub(crate) fn released(ui: &Ui, app: &AppState, metrics: Metrics) -> Vec<Command> {
    let Some(session) = app.session.selected.clone() else {
        return Vec::new();
    };
    let (col, row) = cell(ui.at, ui, metrics);
    vec![Command::Agent(agent::Command::Click {
        session,
        col,
        row,
        down: false,
    })]
}

/// What a selection of our own leaves on the clipboard when the button is let go.
pub(crate) fn copied(app: &AppState) -> Vec<Command> {
    let Some(session) = app.session.selected.clone() else {
        return Vec::new();
    };
    vec![Command::Agent(agent::Command::Copy { session })]
}

/// Whether the program in the agent reads the mouse itself.
fn reads_mouse(app: &AppState, session: &SessionId) -> bool {
    app.agent
        .agent(session)
        .and_then(|one| one.terminal.as_ref())
        .is_some_and(|one| one.reads_mouse())
}

/// The cell of the agent's own grid the point stands on.
fn cell(point: (f32, f32), ui: &Ui, metrics: Metrics) -> (usize, usize) {
    let tokens = metrics.tokens();
    let (x, y) = Layout::of(metrics, ui).agent_origin(&tokens);
    let col = ((point.0 - x) / metrics.cell.width).floor().max(0.0) as usize;
    let row = ((point.1 - y) / metrics.cell.height).floor().max(0.0) as usize;
    (col, row)
}

/// What the agent can be sent, under the word that opened it.
fn menu(ui: &mut Ui, app: &AppState, hits: &Hits, session: SessionId) -> Vec<Command> {
    let Some(open) = app.session.get(&session) else {
        return Vec::new();
    };
    let sources = groove_controllers::task_service::source_ids(app.config.config.as_ref());
    let offered: Vec<Offer> = app
        .agent
        .skills_for(&open.session.kind)
        .iter()
        .flat_map(|one| rows_of(one, &sources))
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

/// One row a skill offers, one per source it files in; `create-task` runs from the chat alone.
fn rows_of(skill: &groove_types::Skill, sources: &[ProviderId]) -> Vec<Offer> {
    if skill.name == "create-task" {
        return Vec::new();
    }
    let files = skill.name == "convert-explorer";
    if !files || sources.len() < 2 {
        return vec![Offer {
            id: skill.id.clone(),
            args: None,
            label: skill.label.clone(),
        }];
    }
    sources
        .iter()
        .map(|one| Offer {
            id: skill.id.clone(),
            args: Some(one.as_str().to_string()),
            label: format!("{} in {}", skill.label, one.label()),
        })
        .collect()
}

/// Every write of the session let through without asking, or asking again.
fn switched(app: &AppState, session: SessionId) -> Vec<Command> {
    let on = !app
        .agent
        .activity(&session)
        .is_some_and(|one| one.auto_approve);
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
