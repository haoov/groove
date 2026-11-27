//! The Kubernetes tab in the modes layout: the list or the object open, under one band on the ground.

pub(crate) mod aside;

use groove_controllers::AppState;
use groove_controllers::session_service::Open;
use groove_gfx::{Edges, Rect};
use groove_types::Attached;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, column_in};
use groove_ui_kit::shape::hairline;
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{Search, Side, Tab, cap, tab_group};

use super::opened::Opened;
use super::{list, pins, scope, tab};
use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;

/// The middle in `area`, a gutter as the conversation's at its sides.
pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui, area: Rect) {
    let column = area.pad(Edges::xy(ctx.tokens.xl, ctx.tokens.sm));
    match ui.session.resources.tab() {
        Some(one) => object(ctx, app, ui, column, one),
        None => listed(ctx, app, ui, column),
    }
}

/// The band, a thin rule under it, and the room left under both.
fn banded(ctx: &mut Ctx, column: Rect) -> (Rect, Rect) {
    let tall = |height: f32| Spec::default().height(height);
    let [band, _, body] = column_in(
        column,
        [tall(ctx.tokens.row), tall(ctx.tokens.sm), Spec::fill()],
    );
    hairline(ctx, band, ctx.styles.panel_edge());
    (band, body)
}

/// The find row over the band while it is up, the list's band, then its rows.
fn listed(ctx: &mut Ctx, app: &AppState, ui: &Ui, mut column: Rect) {
    let held = &ui.session.resources;
    if held.finding || !held.search.is_empty() {
        let search = Search::new(&held.search, Target::ResourceSearch, held.typing);
        let search = search.hint("name, or label=value");
        crate::views::session::diff::finder::row_over(ctx, &mut column, search, None);
    }
    let (band, body) = banded(ctx, column);
    if let Some(open) = app.session.selected() {
        list_band(ctx, app, ui, open, band);
    }
    list::rows(ctx, app, ui, body);
}

/// `PODS · 6`, each context in its hue with its namespaces, then the pin at the right.
fn list_band(ctx: &mut Ctx, app: &AppState, ui: &Ui, open: &Open, band: Rect) {
    let held = &ui.session.resources;
    let mut room = band.pad(Edges::across(ctx.tokens.sm, ctx.tokens.sm));
    let shown = pins::shown(app, open, held);
    if shown.is_some() {
        let pinned = shown.as_ref().is_some_and(|one| open.pins.contains(one));
        let word = if pinned { "pinned" } else { "pin" };
        Tab::new(word, Target::ResourcePin, pinned).right(ctx, &mut room, 0.0);
    }
    if let (Some(kind), count) = list::counted(app, ui) {
        let said = format!("{} · {count}", list::plural(&kind));
        cap(ctx, &mut room, &said, Role::Text);
    }
    let pairs = scope::held(open, held);
    for context in scope::shown_contexts(open, held) {
        chip(ctx, app, &mut room, context, &pairs);
    }
}

/// A context in the hue Settings gives it, then its namespaces, from the left of `room`.
fn chip(ctx: &mut Ctx, app: &AppState, room: &mut Rect, context: &str, pairs: &[Attached]) {
    let (sm, md) = (ctx.tokens.sm, ctx.tokens.md);
    let hue = app.config.cluster(context).map(|one| one.hue);
    let role = hue.map_or(Role::Faint, Role::Hue);
    groove_ui_kit::shape::from_left(room, sm, 0.0);
    Label::new(context, ctx.styles.label(role)).left(ctx, room, sm);
    for one in pairs.iter().filter(|one| one.context == context) {
        let label = match one.namespace.as_deref() {
            Some(name) => Label::new(name, ctx.styles.code(Role::Muted)),
            None => Label::new(super::WHOLE_NAME, ctx.styles.body(Role::Faint)),
        };
        label.left(ctx, room, sm);
    }
    groove_ui_kit::shape::from_left(room, md, 0.0);
}

/// The object's band over its view; the editor's search over the band while it shows.
fn object(ctx: &mut Ctx, app: &AppState, ui: &Ui, mut column: Rect, one: &Opened) {
    if one.view.edits() {
        crate::views::session::diff::finder::above(ctx, &mut column, ui);
    }
    let (band, body) = banded(ctx, column);
    object_band(ctx, app, band, one);
    tab::shown(ctx, body, app, (ui, one));
}

/// `api-0  pod · staging · shop`, its badges, then its views at the right.
fn object_band(ctx: &mut Ctx, app: &AppState, band: Rect, one: &Opened) {
    let object = one.link.read(app).map(|one| &**one);
    let mut room = band.pad(Edges::across(ctx.tokens.sm, ctx.tokens.sm));
    let views: Vec<_> = tab::views(object)
        .iter()
        .map(|view| Tab::new(view.label(), Target::ResourceView(*view), one.view == *view))
        .collect();
    tab_group(ctx, &mut room, views, (Side::Right, ctx.tokens.md));
    let link = &one.link;
    let name = Label::new(&link.name, ctx.styles.file_head(Role::Text));
    name.left(ctx, &mut room, ctx.tokens.md);
    let mut where_ = vec![link.kind.kind.to_lowercase(), link.context.clone()];
    where_.extend(link.namespace.clone());
    let said = where_.join(" · ");
    Label::new(&said, ctx.styles.body(Role::Faint)).left(ctx, &mut room, ctx.tokens.md);
    tab::badges(ctx, room, object);
}
