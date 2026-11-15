//! The column boundaries, drawn by nothing, there for the pointer to find.

use groove_gfx::{Edges, Rect};

use crate::Surface;
use crate::ctx::Ctx;
use crate::hit::Target;
use crate::layout::Edge;

/// A grab band over each boundary the surface shows, above whatever drew there.
/// The feed's top is not dragged while the sessions' window is folded: it rises under its header.
/// With the feed folded too, the agent runs across the rail's column and its edge is not there.
pub fn draw(ctx: &mut Ctx, surface: Surface, (stowed, folded): (bool, bool)) {
    for edge in shown(surface) {
        let over = surface == Surface::Session && stowed && folded;
        if (stowed && *edge == Edge::Feed) || (over && *edge == Edge::Rail) {
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
        Edge::Rail => Some(upright(ctx.app.layout.rail.right())),
        Edge::Agent => {
            (!ctx.app.layout.workspace.is_empty()).then(|| upright(ctx.app.layout.agent.right()))
        }
        // The sidebar's room starts a hairline past its separator, where the edge is grabbed.
        Edge::Sidebar => (!aside.is_empty()).then(|| upright(aside.x - ctx.tokens.hairline)),
        Edge::Manual => {
            let manual = ctx.app.layout.manual;
            let open = manual.h > ctx.tokens.bar;
            open.then(|| groove_ui_kit::shape::band(manual, -grab / 2.0, grab))
        }
        Edge::Feed => {
            let feed = ctx.app.layout.feed;
            let rail = Edges::across(0.0, ctx.tokens.hairline);
            Some(groove_ui_kit::shape::band(feed, -grab / 2.0, grab).pad(rail))
        }
    }
}
