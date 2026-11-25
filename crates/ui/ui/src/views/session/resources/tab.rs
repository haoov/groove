//! One object's tab: what it reads, and the view it shows, described or as YAML.

mod argo;
mod containers;
mod copy;
mod describe;
mod destined;
mod found;
mod grouped;
mod logs;
mod managed;
mod read;
mod relations;
mod revisions;
mod sections;
mod source_band;
mod stages;
mod syncing;
mod wants;
mod written;

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};
use groove_types::Described;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, column_in};
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{Badge, Button, Tab};

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
    let dragged = &ui.session.resources.tab_widths;
    match tab.view {
        View::Yaml => {
            let bar = Spec::default().height(ctx.tokens.row + ctx.tokens.sm);
            let [bar, yaml] = column_in(rest, [bar, Spec::fill()]);
            written::draw(ctx, bar, app, &tab.link.key());
            crate::views::session::diff::editor(ctx, app, ui, yaml)
        }
        View::Logs => logs::draw(ctx, rest, app, (ui, tab)),
        View::Describe => describe::draw(ctx, rest, app, (tab, object), dragged),
        View::Resources => match object.app.as_deref() {
            Some(part) => managed::draw(ctx, rest, (app, tab, part), dragged),
            None => describe::draw(ctx, rest, app, (tab, object), dragged),
        },
        View::Diff => drifted(ctx, rest, (app, ui), tab),
    }
}

/// The status in its colour, then the views.
fn heading(ctx: &mut Ctx, line: Rect, tab: &Opened, object: Option<&Described>) {
    let mut room = line.pad(Edges::across(ctx.tokens.md, ctx.tokens.sm));
    let gap = ctx.tokens.sm;
    let views: &[View] = match object {
        Some(one) if one.pod.is_some() => &[View::Logs, View::Yaml, View::Describe],
        Some(one) if one.app.is_some() => {
            &[View::Yaml, View::Resources, View::Diff, View::Describe]
        }
        _ => &[View::Yaml, View::Describe],
    };
    let shown: Vec<_> = views
        .iter()
        .map(|view| Tab::new(view.label(), Target::ResourceView(*view), tab.view == *view).tight())
        .collect();
    let side = groove_ui_kit::widgets::Side::Right;
    groove_ui_kit::widgets::tab_group(ctx, &mut room, shown, (side, ctx.tokens.xs));
    if object.is_some_and(|one| one.app.is_some()) {
        acts(ctx, &mut room);
    }
    let mut at = room.x;
    let mut said = object.map(stands).unwrap_or_default();
    said.extend(object.and_then(|one| one.app.as_deref()).and_then(revision));
    for (said, role) in said {
        let badge = Badge::new(&said, role);
        if at + badge.width(ctx) > room.right() {
            break;
        }
        at = badge.at(ctx, room, at).right() + gap;
    }
}

/// An Application's SYNC, asked over its diff, and its REFRESH, left of its views.
fn acts(ctx: &mut Ctx, room: &mut Rect) {
    let (gap, ground) = (ctx.tokens.sm, ctx.styles.panel_head());
    let edge = ctx.styles.color(Role::Accent);
    let sync = Button::new("SYNC", Target::ArgoSync, Role::Accent, ground).edged(edge);
    sync.right(ctx, room, gap);
    let refresh = Button::new("REFRESH", Target::ArgoRefresh, Role::Muted, ground);
    refresh.right(ctx, room, gap);
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

/// The commit an Application's first git source on a branch synced to, short.
fn revision(part: &groove_types::AppPart) -> Option<(String, Role)> {
    let sha = part.synced_commit()?;
    Some((groove_types::short_sha(sha), Role::Muted))
}

/// Why an Application on `context` cannot be read from Argo CD: no hub marked, or another one.
pub(super) fn off_hub(app: &AppState, context: &str) -> Option<String> {
    let hub = app.config.clusters().iter().find(|one| one.argo_hub);
    match hub {
        Some(one) if one.context == context => None,
        Some(one) => Some(format!(
            "{context} is not the Argo CD hub; {} is",
            one.context
        )),
        None => Some("no context is the Argo CD hub: mark one in Settings › Clusters".into()),
    }
}

const WAITS: &str = "a cluster write waits on review in the Diff tab";

/// The Application's drift as the diff draws a change, once read for this Application.
fn drifted(ctx: &mut Ctx, body: Rect, (app, ui): (&AppState, &crate::Ui), tab: &Opened) {
    let body = asked_sync(ctx, body, ui, tab);
    let workspace = &app.workspace;
    let key = tab.link.pod();
    let failed = workspace.drift_read.failed.as_ref();
    let off = off_hub(app, &tab.link.context);
    let empty = workspace.stand_in.as_ref().is_some_and(|one| one.empty());
    let (said, role) = match workspace.drift().filter(|(one, _)| **one == key) {
        _ if off.is_some() => (off.unwrap_or_default(), Role::Warn),
        _ if workspace.reviewed().is_some() => (WAITS.to_string(), Role::Attention),
        Some(_) if !empty => return crate::views::session::diff::draw(ctx, app, ui, body),
        Some(_) => ("in sync: nothing differs".to_string(), Role::Ok),
        None => match failed.filter(|(one, _, _)| *one == key) {
            Some((_, _, why)) => (why.clone(), Role::Bad),
            None => ("reading the drift from Argo CD…".to_string(), Role::Working),
        },
    };
    let line = body.pad(Edges::all(ctx.tokens.md));
    let [line, _] = column_in(line, [Spec::default().height(ctx.tokens.row), Spec::fill()]);
    Label::new(&said, ctx.styles.body(role)).draw(ctx, line);
}

/// The sync asked over the diff it would apply, on a band at the top; the room under it.
fn asked_sync(ctx: &mut Ctx, body: Rect, ui: &crate::Ui, tab: &Opened) -> Rect {
    let Some(crate::Overlay::Losing(crate::Losing::Row(written))) = &ui.overlay else {
        return body;
    };
    let syncing = matches!(written.write, super::Write::Sync { .. });
    if !syncing || written.link != tab.link {
        return body;
    }
    let tall = Spec::default().height(ctx.tokens.row + ctx.tokens.sm);
    let [band, rest] = column_in(body, [tall, Spec::fill()]);
    super::written::ask(ctx, band, written);
    rest
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
