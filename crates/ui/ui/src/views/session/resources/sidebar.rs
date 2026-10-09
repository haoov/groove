//! The Resources tab's sidebar: the search, then the kinds the contexts in scope serve, by heading.

use groove_controllers::AppState;
use groove_controllers::session_service::Open;
use groove_gfx::Rect;
use groove_types::{KindHeading, KubeKind};
use groove_ui_kit::base::ground::Ground;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, column_in};
use groove_ui_kit::shape::hairline;
use groove_ui_kit::shape::hoverable;
use groove_ui_kit::text::{elide, row};
use groove_ui_kit::widgets::{Heading, Search, Side, pane};

use super::scope;
use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::{Scroller, Target};

/// One line of the sidebar: a heading, or a kind under it; each a stop of the arrows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Line {
    /// A heading, and whether it stands open.
    Heading(KindHeading, bool),
    Kind(KubeKind),
}

pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui) {
    let rect = ctx.app.layout.sidebar;
    let Some(open) = app.session.selected().filter(|_| !rect.is_empty()) else {
        return;
    };
    pane(ctx, rect, Ground::Band, Side::Left);
    let [search, body] = column_in(rect, [Spec::default().height(ctx.tokens.row), Spec::fill()]);
    searched(ctx, search, ui);
    let picked = scope::kind(app, open, &ui.session.resources);
    let lines = lines(app, open, ui);
    let stood = ui
        .session
        .resources
        .cursor
        .and_then(|at| lines.get(at))
        .cloned();
    let row = ctx.tokens.row;
    crate::offsets::listed(
        ctx,
        body,
        (Scroller::Files, ui.session.files),
        &lines,
        |_| row,
        |ctx, line, one| match one {
            Line::Heading(group, open) => {
                if stood.as_ref() == Some(one) {
                    groove_ui_kit::shape::ground(ctx, line, Ground::Hover);
                }
                let heading = Heading::new(group.label()).fold(*open);
                heading
                    .target(Target::ResourceGroup(group.clone()))
                    .draw(ctx, line)
            }
            Line::Kind(kind) => {
                let marks = (picked.as_ref() == Some(kind), stood.as_ref() == Some(one));
                named(ctx, line, kind, marks)
            }
        },
    );
}

fn searched(ctx: &mut Ctx, line: Rect, ui: &Ui) {
    let held = &ui.session.resources;
    groove_ui_kit::shape::ground(ctx, line, Ground::Work);
    hairline(ctx, line, ctx.styles.line());
    let search = Search::new(&held.filter, Target::ResourceFilter, held.filtering);
    search
        .hint("kind, group or group/kind")
        .faint(Role::Faint)
        .draw(ctx, line);
}

/// One kind at a file's depth: listed, on the held ground between the rules; stood on, hovered.
fn named(ctx: &mut Ctx, line: Rect, kind: &KubeKind, (picked, highlighted): (bool, bool)) {
    if highlighted {
        groove_ui_kit::shape::ground(ctx, line, Ground::Hover);
    }
    hoverable(ctx, line, Target::ResourceKind(kind.clone()));
    if picked {
        groove_ui_kit::shape::chosen(ctx, line);
    }
    let role = if picked { Role::Text } else { Role::Muted };
    let style = ctx.styles.body(role);
    let at = crate::views::session::files::text_at(ctx, ctx.tokens.md * 2.0);
    let room = (line.w - at - ctx.tokens.md).max(0.0);
    let label = super::list::plural(kind);
    let text = elide(ctx, &label, &style, room);
    row(ctx, line, at, &text, style);
}

/// Every watchable kind of the contexts in scope, once each, under its heading.
fn lines(app: &AppState, open: &Open, ui: &Ui) -> Vec<Line> {
    let held = &ui.session.resources;
    let mut kinds: Vec<&KubeKind> = Vec::new();
    for context in scope::contexts(open)
        .into_iter()
        .filter(|one| !held.hidden_contexts.contains(*one))
    {
        let served = app.cluster.store.kinds(context).unwrap_or_default();
        for kind in served.iter().filter(|one| one.watchable) {
            if !kinds
                .iter()
                .any(|one| one.group == kind.group && one.kind == kind.kind)
            {
                kinds.push(kind);
            }
        }
    }
    let filter = held.filter.text().to_lowercase();
    let matches = |one: &&KubeKind| matched(one, &filter);
    let mut headed: std::collections::BTreeMap<KindHeading, Vec<&KubeKind>> = Default::default();
    for one in kinds.into_iter().filter(matches) {
        headed.entry(one.heading()).or_default().push(one);
    }
    let mut out = Vec::new();
    for (heading, mut under) in headed {
        let open = !held.folded.contains(&heading);
        out.push(Line::Heading(heading, open));
        if open {
            under.sort_by(|a, b| a.kind.cmp(&b.kind));
            out.extend(under.into_iter().cloned().map(Line::Kind));
        }
    }
    out
}

/// A word in the kind, its plural, its heading or its API group; `heading/kind` narrows both.
fn matched(one: &KubeKind, filter: &str) -> bool {
    let heading = one.heading().label().to_lowercase();
    let in_place = |place: &str| {
        groove_types::fuzzy(&heading, place) || groove_types::fuzzy(&one.group, place)
    };
    let in_kind =
        |word: &str| groove_types::fuzzy(&one.kind, word) || groove_types::fuzzy(&one.plural, word);
    match filter.split_once('/') {
        Some((place, kind)) => in_place(place) && in_kind(kind),
        None => in_kind(filter) || in_place(filter),
    }
}

/// The sidebar's lines as it shows them: what the arrows step through.
pub fn shown(app: &AppState, ui: &Ui) -> Vec<Line> {
    app.session
        .selected()
        .map(|open| lines(app, open, ui))
        .unwrap_or_default()
}
