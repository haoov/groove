//! A maintenance task's overview: the children it spawned.

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};
use groove_types::Session;
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{flat_row, lead};

use crate::ctx::Ctx;
use crate::hit::Target;

/// The child's kind and title, then what its agent does, or `closed` off the rail.
pub(super) fn child(ctx: &mut Ctx, app: &AppState, rect: Rect, one: &Session) {
    let row = flat_row(ctx, rect, Target::Session(one.id.clone()), false);
    let md = ctx.tokens.md;
    let mut room = row.pad(Edges::across(md * 2.0, md * 2.0));
    let (said, role) = match app.session.get(&one.id) {
        Some(open) => crate::views::rail::state_of(app, open),
        None => ("closed".to_string(), Role::Ghost),
    };
    Label::new(&said, ctx.styles.small(role)).right(ctx, &mut room, md);
    lead(ctx, &mut room, Mark::of_kind(&one.kind), Role::Faint);
    Label::new(&one.title, ctx.styles.label(Role::Text)).draw(ctx, room);
}
