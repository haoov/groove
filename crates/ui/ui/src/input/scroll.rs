//! Where the wheel leaves each column that scrolls.

use groove_controllers::AppState;

use super::Delta;
use crate::ctx::Metrics;
use crate::hit::{Hits, Scroller};
use crate::layout::Layout;
use crate::views::board::List;
use crate::views::session::Tab;
use crate::{Surface, Ui};

/// The column under the pointer scrolls. Wheel down is rows up; the view clamps the
/// far end.
pub(super) fn scroll(
    point: (f32, f32),
    delta: Delta,
    ui: &mut Ui,
    app: &AppState,
    hits: &Hits,
    metrics: Metrics,
) {
    let (x, y) = point;
    let layout = Layout::of(metrics, ui);
    let tokens = metrics.tokens();
    let pixels = |height: f32| delta.down(height);
    if x <= layout.rail.right() {
        let far = hits.extent(Scroller::Rail);
        ui.rail.scroll = moved(ui.rail.scroll, pixels(tokens.row), far);
        return;
    }
    if ui.showing(app) == Surface::Board {
        let band = crate::views::board::bands(&tokens, app, ui, layout.board).timeline;
        if band.contains(x, y) {
            return carry(delta, ui, tokens);
        }
        return column(x, pixels(tokens.row), ui, hits, layout);
    }
    if !layout.sidebar.is_empty() && x >= layout.sidebar.x {
        let far = hits.extent(Scroller::Files);
        ui.session.files = moved(ui.session.files, pixels(tokens.row), far);
        return;
    }
    if x < layout.workspace.x {
        return;
    }
    match ui.session.tab {
        Tab::Overview => {
            let far = hits.extent(Scroller::Overview);
            ui.session.overview = moved(ui.session.overview, pixels(tokens.row), far);
        }
        Tab::File => {
            let far = hits.extent(Scroller::Code);
            ui.session.diff = moved(ui.session.diff, pixels(tokens.line), far);
        }
    }
}

/// A gesture over the band: it carries time only while it goes sideways.
fn carry(delta: Delta, ui: &mut Ui, tokens: crate::tokens::Tokens) {
    let day = crate::tokens::DAY_PIXELS;
    let across = delta.across(day);
    if across.abs() <= delta.down(tokens.row).abs() {
        return;
    }
    ui.board.carry(across);
}

/// The board column the pointer is over.
fn column(x: f32, pixels: f32, ui: &mut Ui, hits: &Hits, layout: Layout) {
    let board = layout.board;
    let width = board.w / List::ALL.len() as f32;
    let at = ((x - board.x) / width.max(1.0)).floor().max(0.0) as usize;
    let Some(list) = List::ALL.get(at).copied() else {
        return;
    };
    let far = hits.extent(Scroller::Column(list as u8));
    let to = moved(ui.board.scroll(list), pixels, far);
    ui.board.scrolled(list, to);
}

/// Where a column stands after the wheel turned, inside what it can scroll.
fn moved(from: f32, pixels: f32, extent: f32) -> f32 {
    (from - pixels).clamp(0.0, extent)
}
