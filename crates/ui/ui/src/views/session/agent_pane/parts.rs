//! What the conversation's blocks share: a ruled box and its head band, a tint, the steps'
//! tally, and who spoke how long ago.

use groove_gfx::{Edges, Rect};
use groove_types::Timestamp;
use groove_types::chat::{Glance, Kind};
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::{Panel, hairline};
use groove_ui_kit::text::{Label, ago};

use crate::ctx::Ctx;
use groove_ui_kit::widgets::cap;

/// A block's hairline box on the pane's own ground.
pub(super) fn boxed(ctx: &mut Ctx, rect: Rect) {
    let panel = Panel::default().ground(ctx.styles.ground());
    panel.border(ctx.styles.panel_edge()).draw(ctx, rect);
}

/// A block's head band across the top of `rect`, ruled under; returns the room inside it.
pub(super) fn head(ctx: &mut Ctx, rect: Rect) -> Rect {
    Panel::default()
        .ground(ctx.styles.panel_head())
        .draw(ctx, rect);
    hairline(ctx, rect, ctx.styles.panel_edge());
    rect.pad(Edges::across(ctx.tokens.md, ctx.tokens.md))
}

/// A block tinted in `role`, as an ask, a failure or a running step stands.
pub(super) fn tinted(ctx: &mut Ctx, rect: Rect, role: Role) {
    let panel = Panel::default().ground(ctx.styles.band_tint(role));
    panel.border(ctx.styles.band_edge(role)).draw(ctx, rect);
}

/// `YOU · 4m`: who spoke in small capitals, then how long ago.
pub(super) fn who(ctx: &mut Ctx, line: Rect, (name, role): (&str, Role), at: Option<Timestamp>) {
    let mut room = line;
    cap(ctx, &mut room, name, role);
    let said = at.map_or_else(String::new, |at| ago(at.age_at(ctx.now)));
    Label::new(&said, ctx.styles.log(Role::Ghost)).draw(ctx, room);
}

/// `Read 3 · Searched 2 · Ran 1`: each kind of step that happened, its word then its count.
pub(super) fn tally(ctx: &mut Ctx, room: &mut Rect, glance: &Glance) {
    let counted = Kind::ALL.map(|kind| (kind, glance.count(kind)));
    let shown = counted.into_iter().filter(|(_, count)| *count > 0);
    for (at, (kind, count)) in shown.enumerate() {
        if at > 0 {
            Label::new("·", ctx.styles.log(Role::Ghost)).left(ctx, room, ctx.tokens.sm);
        }
        let word = Label::new(kind.word(), ctx.styles.tag(Role::Muted));
        word.left(ctx, room, ctx.tokens.xs);
        let count = count.to_string();
        Label::new(&count, ctx.styles.log(Role::Faint)).left(ctx, room, ctx.tokens.sm);
    }
}
