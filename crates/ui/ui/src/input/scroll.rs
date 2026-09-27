//! Where the wheel leaves each column that scrolls.

use groove_controllers::{AppState, Command, agent};

use super::Delta;
use crate::hit::{Hits, Scroller, Target};
use crate::layout::Layout;
use crate::views::board::List;
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
    if x <= layout.rail.right() {
        in_rail(y, pixels(tokens.line), pixels(tokens.row), ui, hits);
        return Vec::new();
    }
    if ui.showing(app) == Surface::Session && x < layout.workspace.x {
        return agent(ui, app, delta, point, &layout, metrics);
    }
    if ui.showing(app) == Surface::Board {
        let band = crate::views::board::bands(&tokens, app, ui, layout.board).timeline;
        if band.contains(x, y) {
            carry(delta, ui, tokens);
        } else {
            column(x, pixels(tokens.row), ui, hits, layout);
        }
        return Vec::new();
    }
    if !layout.sidebar.is_empty() && x >= layout.sidebar.x {
        ui.wheeled(Scroller::Files, pixels(tokens.row), hits);
        return Vec::new();
    }
    match ui.session.tab {
        Tab::Overview => ui.wheeled(Scroller::Overview, pixels(tokens.row), hits),
        Tab::Diff | Tab::Files => ui.wheeled(Scroller::Code, pixels(tokens.line), hits),
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
    let pixels = match delta {
        Delta::Lines { down, .. } => down * tokens.line * groove_ui_kit::base::tokens::NOTCH,
        Delta::Pixels { down, .. } => down,
    };
    let carried = pixels + ui.agent.carried;
    let lines = (carried / tokens.line) as i32;
    ui.agent.carried = carried - lines as f32 * tokens.line;
    if lines == 0 {
        return Vec::new();
    }
    let (origin_x, origin_y) = layout.agent_origin(&tokens);
    let cell = metrics.cell;
    let col = ((point.0 - origin_x) / cell.width).floor().max(0.0) as usize;
    let row = ((point.1 - origin_y) / cell.height).floor().max(0.0) as usize;
    vec![Command::Agent(agent::Command::Scroll {
        session,
        lines,
        col,
        row,
    })]
}

/// A gesture over the band: it carries time only while it goes sideways.
fn carry(delta: Delta, ui: &mut Ui, tokens: groove_ui_kit::base::tokens::Tokens) {
    let day = groove_ui_kit::base::tokens::DAY_PIXELS;
    let across = delta.across(day);
    if across.abs() <= delta.down(tokens.row).abs() {
        return;
    }
    ui.board.carry(across);
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
    let board = layout.board;
    let width = board.w / List::ALL.len() as f32;
    let at = ((x - board.x) / width.max(1.0)).floor().max(0.0) as usize;
    if let Some(list) = List::ALL.get(at).copied() {
        ui.wheeled(Scroller::Column(list as u8), pixels, hits);
    }
}
