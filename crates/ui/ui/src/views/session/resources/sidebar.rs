//! The Resources tab's sidebar: the search, then the kinds the contexts in scope serve, by heading.

use groove_controllers::AppState;
use groove_controllers::session_service::Open;
use groove_gfx::{Edges, Rect};
use groove_types::{KindHeading, KubeKind};
use groove_ui_kit::base::ground::Ground;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, column_in};
use groove_ui_kit::shape::hairline;
use groove_ui_kit::shape::hoverable;
use groove_ui_kit::text::lit_cut;
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
        (Scroller::Kinds, ui.session.resources.kinds),
        &lines,
        |_| row,
        |ctx, line, one| match one {
            Line::Heading(group, open) => {
                if stood.as_ref() == Some(one) {
                    groove_ui_kit::shape::ground(ctx, line, Ground::Raised);
                }
                let heading = Heading::new(group.label()).fold(*open);
                heading
                    .target(Target::ResourceGroup(group.clone()))
                    .draw(ctx, line)
            }
            Line::Kind(kind) => {
                let marks = (picked.as_ref() == Some(kind), stood.as_ref() == Some(one));
                named(ctx, line, (kind, ui.session.resources.filter.text()), marks)
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

/// One kind at a file's depth: listed, on the held ground between the rules; stood on, raised.
fn named(
    ctx: &mut Ctx,
    line: Rect,
    (kind, filter): (&KubeKind, &str),
    (picked, highlighted): (bool, bool),
) {
    if highlighted {
        groove_ui_kit::shape::ground(ctx, line, Ground::Raised);
    }
    hoverable(ctx, line, Target::ResourceKind(kind.clone()));
    if picked {
        groove_ui_kit::shape::chosen(ctx, line);
    }
    let role = if picked { Role::Text } else { Role::Muted };
    let style = ctx.styles.body(role);
    let at = crate::views::session::files::text_at(ctx, ctx.tokens.md * 2.0);
    let room = line.pad(Edges::across(0.0, ctx.tokens.md));
    let label = super::list::plural(kind);
    let word = filter.split_once('/').map_or(filter, |(_, kind)| kind);
    let lit = groove_types::spans(&label, &[word]);
    lit_cut(
        ctx,
        (room, at),
        &label,
        style,
        (&lit, ctx.styles.body(Role::Accent)),
    );
}

/// Every watchable kind of the contexts in scope, once each, under its heading.
fn lines(app: &AppState, open: &Open, ui: &Ui) -> Vec<Line> {
    let held = &ui.session.resources;
    let mut kinds: Vec<&KubeKind> = Vec::new();
    for context in scope::shown_contexts(open, held) {
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
    let mut headed: std::collections::BTreeMap<KindHeading, Vec<(i32, &KubeKind)>> =
        Default::default();
    for one in kinds {
        if let Some(score) = matched(one, &filter) {
            headed.entry(one.heading()).or_default().push((score, one));
        }
    }
    let mut headed: Vec<_> = headed.into_iter().collect();
    for (_, under) in &mut headed {
        under.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.kind.cmp(&b.1.kind)));
    }
    headed.sort_by_key(|(_, under)| std::cmp::Reverse(under.first().map_or(0, |one| one.0)));
    let mut out = Vec::new();
    for (heading, under) in headed {
        let open = !held.folded.contains(&heading);
        out.push(Line::Heading(heading, open));
        if open {
            out.extend(under.into_iter().map(|(_, one)| Line::Kind(one.clone())));
        }
    }
    out
}

/// How well the kind, its plural, its heading or its API group match; `heading/kind` narrows both.
fn matched(one: &KubeKind, filter: &str) -> Option<i32> {
    let heading = one.heading().label().to_lowercase();
    let best = |texts: &[&str], query: &str| {
        let scores = texts
            .iter()
            .filter_map(|text| groove_types::sought(text, query));
        scores.map(|found| found.score).max()
    };
    let (kind, place) = (
        [one.kind.as_str(), one.plural.as_str()],
        [heading.as_str(), one.group.as_str()],
    );
    match filter.split_once('/') {
        Some((there, what)) => best(&place, there).and(best(&kind, what)),
        None => best(&[kind[0], kind[1], place[0], place[1]], filter),
    }
}

/// The sidebar's lines as it shows them: what the arrows step through.
pub fn shown(app: &AppState, ui: &Ui) -> Vec<Line> {
    app.session
        .selected()
        .map(|open| lines(app, open, ui))
        .unwrap_or_default()
}
