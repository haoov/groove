//! The boundaries between the columns. They draw nothing: every pane already carries
//! its own hairline. They exist so the pointer can find them and a drag can move them.

use groove_gfx::Rect;

use crate::ctx::Ctx;
use crate::hit::Target;
use crate::layout::Edge;

/// A grab band over each boundary, over whatever the panes drew there.
pub fn draw(ctx: &mut Ctx) {
    let (grab, height) = (ctx.tokens.grab, ctx.layout.window.h);
    for edge in Edge::ALL {
        let Some(x) = boundary(ctx, edge) else {
            continue;
        };
        let band = Rect::new(x - grab / 2.0, 0.0, grab, height);
        ctx.hit(band, Target::Split(edge));
    }
}

/// Where the boundary was drawn, or nothing when the column it holds is folded.
fn boundary(ctx: &Ctx, edge: Edge) -> Option<f32> {
    match edge {
        Edge::Rail => Some(ctx.layout.rail.right()),
        Edge::Agent => Some(ctx.layout.agent.right()),
        Edge::Sidebar => match ctx.layout.sidebar.is_empty() {
            true => None,
            false => Some(ctx.layout.sidebar.x),
        },
    }
}
