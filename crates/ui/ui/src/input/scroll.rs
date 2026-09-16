//! Where the wheel leaves each column that scrolls.

use super::Delta;
use crate::Ui;
use crate::ctx::Metrics;
use crate::hit::{Hits, Scroller};
use crate::layout::Layout;
use crate::tokens::Tokens;

/// The column under the pointer scrolls. Wheel down is rows up; the view clamps the
/// far end.
pub(super) fn scroll(x: f32, delta: Delta, ui: &mut Ui, hits: &Hits, metrics: Metrics) {
    let layout = Layout::of(metrics, ui);
    let tokens = Tokens::new(metrics.scale);
    let pixels = |height: f32| match delta {
        Delta::Lines(lines) => lines * height,
        Delta::Pixels(pixels) => pixels,
    };
    if x <= layout.rail.right() {
        let far = hits.extent(Scroller::Rail);
        ui.rail.scroll = moved(ui.rail.scroll, pixels(tokens.row), far);
        return;
    }
    if !layout.sidebar.is_empty() && x >= layout.sidebar.x {
        let far = hits.extent(Scroller::Files);
        ui.session.files = moved(ui.session.files, pixels(tokens.row), far);
        return;
    }
    if x >= layout.workspace.x {
        let far = hits.extent(Scroller::Code);
        ui.session.diff = moved(ui.session.diff, pixels(tokens.line), far);
    }
}

/// Where a column stands after the wheel turned, inside what it can scroll.
fn moved(from: f32, pixels: f32, extent: f32) -> f32 {
    (from - pixels).clamp(0.0, extent)
}
