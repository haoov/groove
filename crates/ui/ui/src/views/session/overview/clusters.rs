//! The overview's clusters: each context the session holds in its hue, its namespaces after it.

use groove_controllers::AppState;
use groove_controllers::session_service::Open;
use groove_gfx::{Edges, Rect};
use groove_types::Hue;
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::lead;

use super::Part;
use crate::ctx::Ctx;
use crate::views::session::resources::WHOLE_NAME;

/// A context once, in the order first attached, with each of its namespaces.
pub(super) fn parts<'a>(app: &AppState, open: &'a Open) -> Vec<Part<'a>> {
    crate::views::session::resources::contexts(open)
        .into_iter()
        .map(|context| {
            let hue = app.config.cluster(context).map(|one| one.hue);
            let held = open.clusters.iter().filter(|one| one.context == context);
            Part::Cluster(
                context,
                hue,
                held.map(|one| one.namespace.as_deref()).collect(),
            )
        })
        .collect()
}

/// The context in its hue, faint once Settings no longer knows it, then its namespaces.
pub(super) fn draw(
    ctx: &mut Ctx,
    line: Rect,
    (context, hue): (&str, Option<Hue>),
    held: &[Option<&str>],
) {
    let (sm, md) = (ctx.tokens.sm, ctx.tokens.md);
    let mut room = line.pad(Edges::across(md, md));
    let role = hue.map_or(Role::Faint, Role::Hue);
    lead(ctx, &mut room, Mark::Context, role);
    Label::new(context, ctx.styles.label(role)).left(ctx, &mut room, md);
    for one in held {
        let label = match one {
            Some(name) => Label::new(name, ctx.styles.code(Role::Muted)),
            None => Label::new(WHOLE_NAME, ctx.styles.body(Role::Faint)),
        };
        label.left(ctx, &mut room, sm);
    }
}
