//! The thin ring in the accent around the region that has the keys. The agent has no edge to ring:
//! its caret takes the accent instead.

use groove_controllers::AppState;
use groove_gfx::Rect;

use crate::ctx::Ctx;
use crate::{Focus, Surface, Ui};
use groove_ui_kit::widgets::{aside_bounds, focus_ring};

/// The ring around the focused region's own glass, when it stands open.
pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui) {
    if let Some(rect) = region(ctx, app, ui).filter(|one| !one.is_empty()) {
        focus_ring(ctx, rect);
    }
}

/// What the ring goes around: the sessions' window, the strip, the sidebar or the terminals.
fn region(ctx: &Ctx, app: &AppState, ui: &Ui) -> Option<Rect> {
    let layout = ctx.app.layout;
    let session = ui.showing(app) == Surface::Session && app.session.selected().is_some();
    match ui.focus {
        Focus::Rail => Some(layout.sessions),
        // Folded, the strip's rect is the header's whole line, wider than its capsule: no ring.
        Focus::Workspace if session && !ui.session.stowed => Some(layout.strip),
        // The sidebar's box, its border the separator; none while it is folded away.
        Focus::Sidebar if session && !layout.sidebar.is_empty() => {
            Some(aside_bounds(&ctx.tokens, layout.sidebar))
        }
        Focus::Terminal if session => Some(layout.manual),
        _ => None,
    }
}
