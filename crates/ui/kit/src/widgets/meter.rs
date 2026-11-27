//! How much of something is used, as a thin bar filled from the left.

use groove_gfx::{Align, Rect};

use crate::base::ctx::{App, Ctx};
use crate::base::style::Role;

/// `share` of a bar `xs` tall, centred down `rect`, filled in `role`'s colour.
pub fn meter<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect, share: f32, role: Role) {
    let track = rect.align((rect.w, ctx.tokens.xs), Align::Start, Align::Center);
    ctx.quad(track, ctx.styles.line());
    let filled = track.w * share.clamp(0.0, 1.0);
    let fill = track.align((filled, track.h), Align::Start, Align::Center);
    ctx.quad(fill, ctx.styles.color(role));
}
