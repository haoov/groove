//! The boundaries between the columns. They draw nothing: every pane already carries
//! its own hairline. They exist so the pointer can find them and a drag can move them.

use groove_gfx::Rect;

use crate::ctx::Ctx;
use crate::hit::Target;
use crate::layout::Edge;

/// A grab band over each boundary, over whatever the panes drew there.
pub fn draw(ctx: &mut Ctx) {
    let (grab, height) = (ctx.tokens.grab, ctx.layout.window.h);
    let edges = [
        (Edge::Rail, ctx.layout.rail.right()),
        (Edge::Agent, ctx.layout.agent.right()),
    ];
    for (edge, x) in edges {
        let band = Rect::new(x - grab / 2.0, 0.0, grab, height);
        ctx.hit(band, Target::Split(edge));
    }
}
