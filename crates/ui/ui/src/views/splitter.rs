//! The column boundaries, drawn by nothing, there for the pointer to find.

use groove_gfx::Rect;

use crate::Surface;
use crate::ctx::Ctx;
use crate::hit::Target;
use crate::layout::Edge;

/// A grab band over each boundary the surface shows, above whatever drew there.
pub fn draw(ctx: &mut Ctx, surface: Surface) {
    for edge in shown(surface) {
        let Some(band) = band(ctx, *edge) else {
            continue;
        };
        ctx.hit(band, Target::Split(*edge));
    }
}

/// The boundaries a surface has: the board has only the rail's.
fn shown(surface: Surface) -> &'static [Edge] {
    match surface {
        Surface::Session => &Edge::ALL,
        Surface::Board => &[Edge::Rail],
    }
}

/// The band to grab a boundary by, or nothing when its column is folded.
fn band(ctx: &Ctx, edge: Edge) -> Option<Rect> {
    let (grab, window) = (ctx.tokens.grab, ctx.window);
    let upright = |x: f32| Rect::new(x - grab / 2.0, 0.0, grab, window.h);
    let aside = ctx.app.layout.sidebar;
    match edge {
        Edge::Rail => Some(upright(ctx.app.layout.rail.right())),
        Edge::Agent => Some(upright(ctx.app.layout.agent.right())),
        Edge::Sidebar => (!aside.is_empty()).then(|| upright(aside.x)),
        Edge::Commit => (!aside.is_empty()).then(|| {
            let top = ctx.app.layout.commit.y;
            Rect::new(aside.x, top - grab / 2.0, aside.w, grab)
        }),
        Edge::Manual => {
            let manual = ctx.app.layout.manual;
            let open = manual.h > ctx.tokens.bar;
            open.then(|| Rect::new(manual.x, manual.y - grab / 2.0, manual.w, grab))
        }
        Edge::Band | Edge::Feed => None,
    }
}
