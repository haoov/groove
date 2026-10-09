//! One object's tab: what it reads, and the view it shows, described or as YAML.

mod card;
mod describe;
mod relations;
mod sections;
mod wants;

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};
use groove_types::{Described, Health};
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, column_in};
use groove_ui_kit::shape::hairline;
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{Badge, Tab, Text};

pub use wants::{due, wants};

use super::list::plan::colour;
use super::opened::Opened;
use crate::ctx::Ctx;
use crate::hit::Target;

pub fn draw(ctx: &mut Ctx, body: Rect, app: &AppState, (ui, tab): (&crate::Ui, &Opened)) {
    let tall = Spec::default().height(ctx.tokens.row + ctx.tokens.sm * 2.0);
    let [top, rest] = column_in(body, [tall, Spec::fill()]);
    let object = tab.link.read(app);
    heading(ctx, top, app, tab, object.map(|one| &**one));
    let Some(object) = object else {
        return standing(ctx, rest, app, tab);
    };
    match tab.yaml {
        true => crate::views::session::diff::editor(ctx, app, ui, rest),
        false => describe::draw(ctx, rest, app, (tab, object)),
    }
}

/// Context, namespace, kind and name, the status in its colour, then the two views.
fn heading(ctx: &mut Ctx, line: Rect, app: &AppState, tab: &Opened, object: Option<&Described>) {
    hairline(ctx, line, ctx.styles.line());
    let link = &tab.link;
    let mut room = line.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
    let gap = ctx.tokens.sm;
    for (yaml, label) in [(true, "yaml"), (false, "describe")] {
        Tab::new(label, Target::ResourceView(yaml), tab.yaml == yaml)
            .text(Text::Small)
            .tight()
            .right(ctx, &mut room, 0.0);
    }
    let hue = app
        .config
        .cluster(&link.context)
        .map(|one| Role::Hue(one.hue));
    let code = |role| ctx.styles.small(role);
    let (context, faint, muted, text) = (
        code(hue.unwrap_or(Role::Muted)),
        code(Role::Ghost),
        code(Role::Muted),
        code(Role::Text),
    );
    Label::new(&link.context, context).left(ctx, &mut room, gap);
    if let Some(namespace) = &link.namespace {
        Label::new("›", faint).left(ctx, &mut room, gap);
        Label::new(namespace, muted).left(ctx, &mut room, gap);
    }
    Label::new("›", faint).left(ctx, &mut room, gap);
    Label::new(&link.kind.kind, ctx.styles.small(Role::Faint)).left(ctx, &mut room, gap);
    Label::new(&link.name, text).left(ctx, &mut room, gap);
    if let Some((said, role)) = object.and_then(stands) {
        Badge::new(&said, role).at(ctx, room, room.x);
    }
}

/// What the object says of itself: a pod's status, a workload's ready replicas.
fn stands(object: &Described) -> Option<(String, Role)> {
    if let Some(pod) = &object.pod {
        let health = groove_types::status_health(&pod.status).unwrap_or(Health::Waiting);
        return Some((pod.status.clone(), colour(health)));
    }
    let (ready, desired) = object.replicas?;
    let role = match ready >= desired {
        true => Role::Ok,
        false => Role::Warn,
    };
    Some((format!("{ready}/{desired} ready"), role))
}

/// Why the tab holds nothing yet: the object being read, or gone, or the read refused.
fn standing(ctx: &mut Ctx, body: Rect, app: &AppState, tab: &Opened) {
    let held = app.cluster.store.follows.get(&tab.link.key());
    let (said, role) = match held {
        Some(one) if one.failed.is_some() => (one.failed.clone().unwrap_or_default(), Role::Bad),
        Some(one) if one.synced => ("this object is gone".to_string(), Role::Faint),
        _ => (format!("reading {}…", tab.link.name), Role::Working),
    };
    let line = body.pad(Edges::all(ctx.tokens.md));
    let [line, _] = column_in(line, [Spec::default().height(ctx.tokens.row), Spec::fill()]);
    Label::new(&said, ctx.styles.body(role)).draw(ctx, line);
}
