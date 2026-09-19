//! The boundaries a press takes hold of, and how far the pointer has carried one.

use crate::ctx::Metrics;
use crate::layout::Edge;
use crate::tokens::{CLICK_MS, CLICK_SLOP};
use crate::{Click, Drag, Ui};

/// This press, against the one before it: a press soon after another and near it
/// carries the same click on.
pub(super) fn counted(last: Option<Click>, x: f32, y: f32, metrics: Metrics) -> Click {
    let slop = CLICK_SLOP * metrics.scale;
    let same = last.filter(|last| {
        metrics.tick.saturating_sub(last.at) <= CLICK_MS
            && (last.x - x).abs() <= slop
            && (last.y - y).abs() <= slop
    });
    Click {
        x,
        y,
        at: metrics.tick,
        count: same.map_or(1, |last| last.count + 1),
    }
}

/// Takes hold of `edge`, keeping how far from it the pointer landed.
pub(super) fn grab(ui: &mut Ui, edge: Edge, x: f32, y: f32, metrics: Metrics) {
    let at = ui.split.edge_at(edge, window_of(metrics), sidebar(ui));
    ui.drag = Some(Drag {
        edge,
        offset: along(edge, x, y, metrics) - at,
    });
}

/// The pointer's place along the axis the boundary moves in, in logical pixels.
pub(super) fn along(edge: Edge, x: f32, y: f32, metrics: Metrics) -> f32 {
    match edge.upright() {
        true => logical(x, metrics),
        false => logical(y, metrics),
    }
}

/// The boundary follows the pointer.
pub(super) fn drag_to(ui: &mut Ui, x: f32, y: f32, metrics: Metrics) {
    let Some(drag) = ui.drag else {
        return;
    };
    let at = along(drag.edge, x, y, metrics) - drag.offset;
    ui.split
        .drag(drag.edge, at, window_of(metrics), sidebar(ui));
}

pub(super) fn sidebar(ui: &Ui) -> bool {
    ui.session.sidebar()
}

/// The window's width in logical pixels.
pub(super) fn window_of(metrics: Metrics) -> (f32, f32) {
    let rect = metrics.size.rect();
    (logical(rect.w, metrics), logical(rect.h, metrics))
}

pub(super) fn logical(value: f32, metrics: Metrics) -> f32 {
    value / metrics.scale
}
