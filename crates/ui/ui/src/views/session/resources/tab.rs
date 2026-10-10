//! One object's tab: what it reads, and the view it shows, described or as YAML.

mod argo;
mod card;
mod copy;
mod describe;
mod logs;
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

use super::View;
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
    match tab.view {
        View::Yaml => crate::views::session::diff::editor(ctx, app, ui, rest),
        View::Logs => logs::draw(ctx, rest, app, (ui, tab)),
        View::Describe => describe::draw(ctx, rest, app, (tab, object)),
    }
}

/// Context, namespace, kind and name, the status in its colour, then the two views.
fn heading(ctx: &mut Ctx, line: Rect, app: &AppState, tab: &Opened, object: Option<&Described>) {
    hairline(ctx, line, ctx.styles.line());
    let link = &tab.link;
    let mut room = line.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
    let gap = ctx.tokens.sm;
    let views: &[View] = match object.is_some_and(|one| one.pod.is_some()) {
        true => &[View::Logs, View::Yaml, View::Describe],
        false => &[View::Yaml, View::Describe],
    };
    for view in views {
        Tab::new(view.label(), Target::ResourceView(*view), tab.view == *view)
            .text(Text::Small)
            .tight()
            .right(ctx, &mut room, 0.0);
    }
    let hue = app
        .config
        .cluster(&link.context)
        .map(|one| Role::Hue(one.hue));
    let faint = ctx.styles.small(Role::Ghost);
    let last = tab.copied.as_deref();
    let copied = |text, role| copy::Copied::new(text, role, last);
    copied(&link.context, hue.unwrap_or(Role::Muted)).left(ctx, &mut room, gap);
    if let Some(namespace) = &link.namespace {
        Label::new("›", faint).left(ctx, &mut room, gap);
        copied(namespace, Role::Muted).left(ctx, &mut room, gap);
    }
    Label::new("›", faint).left(ctx, &mut room, gap);
    Label::new(&link.kind.kind, ctx.styles.small(Role::Faint)).left(ctx, &mut room, gap);
    copied(&link.name, Role::Text).left(ctx, &mut room, gap);
    let mut at = room.x;
    for (said, role) in object.map(stands).unwrap_or_default() {
        at = Badge::new(&said, role).at(ctx, room, at).right() + ctx.tokens.sm;
    }
}

/// What the object says of itself: a pod's status, a workload's ready replicas, an Application's sync and health.
fn stands(object: &Described) -> Vec<(String, Role)> {
    let worded = |word: &str| {
        let health = groove_types::status_health(word).unwrap_or(Health::Waiting);
        (word.to_string(), colour(health))
    };
    if let Some(pod) = &object.pod {
        return vec![worded(&pod.status)];
    }
    if let Some(app) = &object.app {
        let said = [&app.sync, &app.health]
            .into_iter()
            .filter(|one| !one.is_empty());
        return said.map(|one| worded(one)).collect();
    }
    let Some((ready, desired)) = object.replicas else {
        return Vec::new();
    };
    let role = match ready >= desired {
        true => Role::Ok,
        false => Role::Warn,
    };
    vec![(format!("{ready}/{desired} ready"), role)]
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
