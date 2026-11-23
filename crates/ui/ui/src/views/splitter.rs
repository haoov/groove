//! The column boundaries, drawn by nothing, there for the pointer to find.

use groove_gfx::{Edges, Rect};

use crate::Surface;
use crate::ctx::Ctx;
use crate::hit::Target;
use crate::layout::Edge;

/// A grab band over each boundary the surface shows, above whatever drew there.
/// With both the rail's windows folded the agent, or the board, runs across the rail's column:
/// neither the rail's edge nor the one between its windows is there.
pub fn draw(ctx: &mut Ctx, surface: Surface, (stowed, folded): (bool, bool)) {
    for edge in shown(surface) {
        // With either rail window folded the other takes the column: nothing to drag between.
        let rail = stowed && folded && *edge == Edge::Rail;
        if rail || ((stowed || folded) && *edge == Edge::Feed) {
            continue;
        }
        let Some(band) = band(ctx, *edge) else {
            continue;
        };
        ctx.hit(band, Target::Split(*edge));
    }
}

/// The boundaries a surface has: the board has only the rail's.
fn shown(surface: Surface) -> &'static [Edge] {
    match surface {
        Surface::Session => &[
            Edge::Rail,
            Edge::Agent,
            Edge::Sidebar,
            Edge::Manual,
            Edge::Feed,
            Edge::Alerts,
        ],
        Surface::Board => &[Edge::Rail, Edge::Feed],
    }
}

/// The band to grab a boundary by, or nothing when its column is folded.
fn band(ctx: &Ctx, edge: Edge) -> Option<Rect> {
    let (grab, window) = (ctx.tokens.grab, ctx.window);
    let upright = |x: f32| groove_ui_kit::shape::strip(window, x - grab / 2.0, grab);
    let aside = ctx.app.layout.sidebar;
    match edge {
        // Each band stands on the border the user sees: the rail's windows' right edge, the
        // strip's left one, not the gap's far side.
        Edge::Rail => Some(upright(ctx.app.layout.sessions.right())),
        // The strip's column stands while its workspace or its terminals are open.
        Edge::Agent => {
            let layout = &ctx.app.layout;
            let column = !layout.workspace.is_empty() || layout.manual.h > ctx.tokens.bar;
            column.then(|| upright(layout.strip.x))
        }
        // The sidebar's room starts a hairline past its separator, where the edge is grabbed.
        Edge::Sidebar => (!aside.is_empty()).then(|| upright(aside.x - ctx.tokens.hairline)),
        Edge::Alerts => {
            let alerts = ctx.app.layout.alerts;
            let open = alerts.h > ctx.tokens.bar;
            open.then(|| groove_ui_kit::shape::band(alerts, alerts.h - grab / 2.0, grab))
        }
        Edge::Manual => {
            let manual = ctx.app.layout.manual;
            // Under a folded strip the drawer reaches its header: no line to drag.
            let open = manual.h > ctx.tokens.bar && !ctx.app.layout.workspace.is_empty();
            open.then(|| groove_ui_kit::shape::band(manual, -grab / 2.0, grab))
        }
        // On the feed's top, both windows open.
        Edge::Feed => {
            let (sessions, feed) = (ctx.app.layout.sessions, ctx.app.layout.feed);
            let rail = Edges::across(0.0, ctx.tokens.hairline);
            let at = feed.y - sessions.y;
            Some(groove_ui_kit::shape::band(sessions, at - grab / 2.0, grab).pad(rail))
        }
    }
}
