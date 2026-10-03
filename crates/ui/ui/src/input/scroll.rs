//! Where the wheel leaves each column that scrolls.

use groove_controllers::{AppState, Command, agent};

use super::Delta;
use crate::hit::{Hits, Scroller, Target};
use crate::layout::Layout;
use crate::views::session::Tab;
use crate::{Surface, Ui};
use groove_ui_kit::base::ctx::Metrics;

/// The column under the pointer scrolls; wheel down is rows up.
pub(super) fn scroll(
    point: (f32, f32),
    delta: Delta,
    ui: &mut Ui,
    app: &AppState,
    hits: &Hits,
    metrics: Metrics,
) -> Vec<Command> {
    let (x, y) = point;
    let layout = Layout::of(metrics, ui);
    let tokens = metrics.tokens();
    let pixels = |height: f32| delta.down(height);
    if ui.settings.open {
        ui.wheeled(Scroller::Settings, pixels(tokens.row), hits);
        return Vec::new();
    }
    if x <= layout.rail.right() {
        in_rail(y, pixels(tokens.line), pixels(tokens.row), ui, hits);
        return Vec::new();
    }
    if ui.showing(app) == Surface::Session && x < layout.workspace.x {
        return agent(ui, app, delta, point, &layout, metrics);
    }
    if ui.showing(app) == Surface::Session && layout.manual.contains(x, y) {
        return shelled(ui, app, delta, point, hits, metrics);
    }
    if ui.showing(app) == Surface::Board {
        column(x, pixels(tokens.row), ui, hits, layout);
        return Vec::new();
    }
    if !layout.sidebar.is_empty() && x >= layout.sidebar.x {
        ui.wheeled(Scroller::Files, pixels(tokens.row), hits);
        return Vec::new();
    }
    match ui.session.tab {
        Tab::Overview => ui.wheeled(Scroller::Overview, pixels(tokens.row), hits),
        Tab::Diff | Tab::Files => {
            ui.wheeled(Scroller::Code, pixels(tokens.line), hits);
            ui.wheeled(Scroller::Across, delta.across(tokens.line), hits);
        }
    }
    Vec::new()
}

/// The wheel over the agent's screen. What is not yet worth a line is carried on.
fn agent(
    ui: &mut Ui,
    app: &AppState,
    delta: Delta,
    point: (f32, f32),
    layout: &Layout,
    metrics: Metrics,
) -> Vec<Command> {
    let Some(session) = app.session.selected.clone() else {
        return Vec::new();
    };
    let tokens = metrics.tokens();
    let lines = turned(ui, delta, tokens.line);
    if lines == 0 {
        return Vec::new();
    }
    let origin = layout.agent_origin(&tokens);
    let (col, row) = crate::layout::cell_at(origin, metrics.cell, point);
    vec![Command::Agent(agent::Command::Scroll {
        session,
        lines,
        col,
        row,
    })]
}

/// The wheel over one terminal of the manual section, at the cell it stands on.
fn shelled(
    ui: &mut Ui,
    app: &AppState,
    delta: Delta,
    point: (f32, f32),
    hits: &Hits,
    metrics: Metrics,
) -> Vec<Command> {
    let (Some(session), Some(Target::Shell(id))) =
        (app.session.selected.clone(), hits.at(point.0, point.1))
    else {
        return Vec::new();
    };
    let Some(pane) = hits.rect_of(&Target::Shell(id)) else {
        return Vec::new();
    };
    let tokens = metrics.tokens();
    let lines = turned(ui, delta, tokens.line);
    if lines == 0 {
        return Vec::new();
    }
    let origin = (pane.x + tokens.sm, pane.y + tokens.sm);
    let (col, row) = crate::layout::cell_at(origin, metrics.cell, point);
    let scroll = groove_controllers::shell::Command::Scroll {
        session,
        id,
        lines,
        col,
        row,
    };
    vec![Command::Shell(scroll)]
}

/// Whole lines the wheel has turned; what is not yet worth a line is carried on.
fn turned(ui: &mut Ui, delta: Delta, line: f32) -> i32 {
    let pixels = match delta {
        Delta::Lines { down, .. } => down * line * groove_ui_kit::base::tokens::NOTCH,
        Delta::Pixels { down, .. } => down,
    };
    let carried = pixels + ui.agent.carried;
    let lines = (carried / line) as i32;
    ui.agent.carried = carried - lines as f32 * line;
    lines
}

/// The rail's rows, or the feed under them.
fn in_rail(y: f32, lines: f32, rows: f32, ui: &mut Ui, hits: &Hits) {
    let on_feed = hits
        .rect_of(&Target::Feed)
        .is_some_and(|head| y >= head.y && !ui.rail.folded);
    match on_feed {
        true => ui.wheeled(Scroller::Feed, lines, hits),
        false => ui.wheeled(Scroller::Rail, rows, hits),
    }
}

fn column(x: f32, pixels: f32, ui: &mut Ui, hits: &Hits, layout: Layout) {
    if let Some(list) = crate::views::board::list_at(layout.board, x) {
        ui.wheeled(Scroller::Column(list as u8), pixels, hits);
    }
}
