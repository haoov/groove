//! One object's tab: what it reads, and the view it shows, described or as YAML.

mod argo;
mod containers;
mod copy;
mod describe;
mod destined;
mod grouped;
mod logs;
mod managed;
mod relations;
mod sections;
mod stages;
mod syncing;
mod wants;

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};
use groove_types::Described;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, column_in};
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{Badge, Tab, Text};

pub use wants::{due, wants};

use super::View;
use super::health::{colour, status_colour};
use super::opened::Opened;
use crate::ctx::Ctx;
use crate::hit::Target;

pub fn draw(ctx: &mut Ctx, body: Rect, app: &AppState, (ui, tab): (&crate::Ui, &Opened)) {
    let tall = Spec::default().height(ctx.tokens.row + ctx.tokens.sm * 2.0);
    let [top, rest] = column_in(body, [tall, Spec::fill()]);
    let object = tab.link.read(app);
    heading(ctx, top, tab, object.map(|one| &**one));
    let Some(object) = object else {
        return standing(ctx, rest, app, tab);
    };
    match tab.view {
        View::Yaml => crate::views::session::diff::editor(ctx, app, ui, rest),
        View::Logs => logs::draw(ctx, rest, app, (ui, tab)),
        View::Describe => describe::draw(ctx, rest, app, (tab, object)),
        View::Resources => match object.app.as_deref() {
            Some(part) => {
                let dragged = &ui.session.resources.tab_widths;
                managed::draw(ctx, rest, (app, tab, part), dragged)
            }
            None => describe::draw(ctx, rest, app, (tab, object)),
        },
    }
}

/// The status in its colour, then the views.
fn heading(ctx: &mut Ctx, line: Rect, tab: &Opened, object: Option<&Described>) {
    let mut room = line.pad(Edges::across(ctx.tokens.md, ctx.tokens.sm));
    let gap = ctx.tokens.sm;
    let views: &[View] = match object {
        Some(one) if one.pod.is_some() => &[View::Logs, View::Yaml, View::Describe],
        Some(one) if one.app.is_some() => &[View::Yaml, View::Resources, View::Describe],
        _ => &[View::Yaml, View::Describe],
    };
    for view in views {
        let shown = Tab::new(view.label(), Target::ResourceView(*view), tab.view == *view);
        let shown = shown.text(Text::Small).tight();
        shown.right(ctx, &mut room, ctx.tokens.xs);
    }
    let mut at = room.x;
    for (said, role) in object.map(stands).unwrap_or_default() {
        let badge = Badge::new(&said, role);
        if at + badge.width(ctx) > room.right() {
            break;
        }
        at = badge.at(ctx, room, at).right() + gap;
    }
}

/// What the object says of itself: a pod's status, a workload's ready replicas, an Application's sync and health.
fn stands(object: &Described) -> Vec<(String, Role)> {
    let worded = |word: &str| (word.to_string(), status_colour(word));
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
    let role = colour(groove_types::count_health(ready, desired));
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
