//! The overview's clusters: each context the session holds in its hue, its namespaces under it.

use groove_controllers::AppState;
use groove_controllers::session_service::Open;
use groove_gfx::Rect;
use groove_types::Hue;
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::after_mark;
use groove_ui_kit::widgets::{Row, list};

use super::Part;
use crate::ctx::Ctx;

/// A context once, in the order first attached, then each of its namespaces.
pub(super) fn parts<'a>(app: &AppState, open: &'a Open) -> Vec<Part<'a>> {
    let mut contexts: Vec<&str> = Vec::new();
    for one in &open.clusters {
        if !contexts.contains(&one.context.as_str()) {
            contexts.push(&one.context);
        }
    }
    let mut out = Vec::new();
    for context in contexts {
        let hue = app.config.cluster(context).map(|one| one.hue);
        out.push(Part::Cluster(context, hue));
        let held = open.clusters.iter().filter(|one| one.context == context);
        out.extend(held.map(|one| Part::Namespace(one.namespace.as_deref())));
        out.push(Part::Gap);
    }
    out
}

/// A context Settings no longer knows stands in the faint role.
pub(super) fn context(ctx: &mut Ctx, line: Rect, context: &str, hue: Option<Hue>) {
    let role = hue.map_or(Role::Faint, Role::Hue);
    let row = Row::new(ctx.tokens.md, context, ctx.styles.label(role)).mark(Mark::Context);
    list(ctx, line, &[row], None);
}

pub(super) fn namespace(ctx: &mut Ctx, line: Rect, namespace: Option<&str>) {
    let indent = after_mark(ctx, ctx.tokens.md);
    let row = match namespace {
        Some(one) => Row::new(indent, one, ctx.styles.code(Role::Muted)),
        None => Row::new(indent, "whole cluster", ctx.styles.body(Role::Faint)),
    };
    list(ctx, line, &[row], None);
}
