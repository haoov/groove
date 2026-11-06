//! How much of something is used, as a ring of dots filled clockwise.

use groove_gfx::{Color, Rect};

use crate::base::ctx::{App, Ctx};

use crate::base::tokens::GAUGE_DOT;

/// How many dots a gauge's ring is made of.
const GAUGE_DOTS: usize = 16;

/// `share` of a ring of dots in `rect`, filled clockwise from the top.
pub fn gauge_ring<A: App>(
    ctx: &mut Ctx<'_, A>,
    rect: Rect,
    share: f32,
    (full, empty): (Color, Color),
) {
    let size = rect.w.min(rect.h);
    let dot = size * GAUGE_DOT;
    let reach = (size - dot) / 2.0;
    let centre = (rect.x + rect.w / 2.0, rect.y + rect.h / 2.0);
    let lit = (share.clamp(0.0, 1.0) * GAUGE_DOTS as f32).round() as usize;
    for at in 0..GAUGE_DOTS {
        let turn = at as f32 / GAUGE_DOTS as f32 * std::f32::consts::TAU;
        let (x, y) = (centre.0 + reach * turn.sin(), centre.1 - reach * turn.cos());
        let one = Rect::new(x - dot / 2.0, y - dot / 2.0, dot, dot);
        let color = if at < lit { full } else { empty };
        ctx.rounded(one, color, dot / 2.0);
    }
}
