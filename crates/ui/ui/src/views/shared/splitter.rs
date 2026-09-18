//! The boundaries between the columns. They draw nothing; every pane carries its own
//! hairline. They are here for the pointer to find.

use groove_gfx::Rect;

use crate::ctx::Ctx;
use crate::hit::Target;
use crate::layout::Edge;

/// A grab band over each boundary, above whatever the panes drew there.
pub fn draw(ctx: &mut Ctx) {
    for edge in Edge::ALL {
        let Some(band) = band(ctx, edge) else {
            continue;
        };
        ctx.hit(band, Target::Split(edge));
    }
}

/// The band to grab a boundary by, or nothing when its column is folded.
fn band(ctx: &Ctx, edge: Edge) -> Option<Rect> {
    let (grab, window) = (ctx.tokens.grab, ctx.layout.window);
    let upright = |x: f32| Rect::new(x - grab / 2.0, 0.0, grab, window.h);
    let aside = ctx.layout.sidebar;
    match edge {
        Edge::Rail => Some(upright(ctx.layout.rail.right())),
        Edge::Agent => Some(upright(ctx.layout.agent.right())),
        Edge::Sidebar => (!aside.is_empty()).then(|| upright(aside.x)),
        Edge::Commit => (!aside.is_empty()).then(|| {
            let top = ctx.layout.commit.y;
            Rect::new(aside.x, top - grab / 2.0, aside.w, grab)
        }),
    }
}
