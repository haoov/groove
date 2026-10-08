//! The Resources tab's panel: the list tab, titled by its kind and count, over the rows.

use groove_controllers::AppState;
use groove_controllers::session_service::Open;
use groove_gfx::{Edges, Rect};
use groove_types::{Health, KubeKind, ObjectRow, WatchKey};
use groove_ui_kit::base::ground::Ground;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, column_in};
use groove_ui_kit::shape::hairline;
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{Cell, Column, Rows, Search, Shown, Sorted, Tab, Table, Text, Width};

mod order;
mod plan;

use self::plan::{Plan, Reads, colour, plan};
use super::scope::{self, ResourcesUi};
use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::{Scroller, Target};

pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui, body: Rect) {
    let Some(open) = app.session.selected() else {
        return;
    };
    let held = &ui.session.resources;
    let band = Spec::default().height(ctx.tokens.row + ctx.tokens.sm);
    let [strip, mut body] = column_in(body, [band, Spec::fill()]);
    if held.finding || !held.search.is_empty() {
        let [bar, rest] = column_in(body, [Spec::default().height(ctx.tokens.row), Spec::fill()]);
        found(ctx, bar, held);
        body = rest;
    }
    let keys = scope::keys(app, open, held);
    let (_, names) = scope::search(held);
    let rows = rows(app, &keys, &names);
    let kind = scope::kind(app, open, held);
    title(ctx, strip, kind.as_ref(), rows.len());
    if let Some((said, role)) = standing(app, open, held, &keys) {
        let line = body.pad(Edges::all(ctx.tokens.md));
        let [line, _] = column_in(line, [Spec::default().height(ctx.tokens.row), Spec::fill()]);
        Label::new(&said, ctx.styles.body(role)).draw(ctx, line);
        return;
    }
    let failing = keys
        .iter()
        .filter_map(|key| app.cluster.store.watched(key))
        .find_map(|one| one.failed.as_deref());
    if let Some(why) = failing {
        let [line, rest] = column_in(body, [Spec::default().height(ctx.tokens.row), Spec::fill()]);
        let said = format!("not live: {why} · trying again");
        Label::new(&said, ctx.styles.small(Role::Warn))
            .draw(ctx, line.pad(Edges::across(ctx.tokens.md, ctx.tokens.md)));
        body = rest;
    }
    let kind = kind.map(|one| one.plural).unwrap_or_default();
    listed(ctx, body, app, (&keys, rows), (held, &kind));
}

/// The find bar over the rows: what it narrows by, typed.
fn found(ctx: &mut Ctx, line: Rect, held: &ResourcesUi) {
    groove_ui_kit::shape::ground(ctx, line, Ground::Work);
    hairline(ctx, line, ctx.styles.line());
    let search = Search::new(&held.search, Target::ResourceSearch, held.typing);
    search
        .hint("name, or label=value")
        .code()
        .faint(Role::Faint)
        .draw(ctx, line);
}

/// The list's own tab: the kind and how many rows it holds.
fn title(ctx: &mut Ctx, strip: Rect, kind: Option<&KubeKind>, count: usize) {
    groove_ui_kit::shape::ground(ctx, strip, Ground::Band);
    hairline(ctx, strip, ctx.styles.line());
    let Some(kind) = kind else {
        return;
    };
    let named = format!("{} · {count}", plural(kind));
    let mut room = strip;
    let ground = ctx.styles.ground();
    Tab::new(&named, Target::ResourceList, true)
        .text(Text::Small)
        .quiet(Role::Muted)
        .ground(ground)
        .left(ctx, &mut room, 0.0);
}

/// `Pods`, from the kind's plural.
pub(super) fn plural(kind: &KubeKind) -> String {
    let mut letters = kind.plural.chars();
    letters
        .next()
        .map(|first| first.to_uppercase().chain(letters).collect())
        .unwrap_or_default()
}

/// Why no row shows yet, while none can: no kinds yet, a kind not served, a list pending or failed.
fn standing(
    app: &AppState,
    open: &Open,
    held: &ResourcesUi,
    keys: &[WatchKey],
) -> Option<(String, Role)> {
    let store = &app.cluster.store;
    if scope::held(open, held).next().is_none() {
        return Some((
            "nothing picked: pick a context and a namespace above".into(),
            Role::Faint,
        ));
    }
    let kind = match scope::kind(app, open, held) {
        Some(kind) => kind,
        None => return Some(("reading the cluster's kinds…".into(), Role::Working)),
    };
    if keys.is_empty() {
        return Some((
            format!("no cluster in scope serves {}", kind.plural),
            Role::Faint,
        ));
    }
    let watched: Vec<_> = keys.iter().filter_map(|key| store.watched(key)).collect();
    if watched.iter().any(|one| one.synced) {
        return None;
    }
    match watched.iter().find_map(|one| one.failed.as_deref()) {
        Some(why) => Some((why.to_string(), Role::Bad)),
        None => Some((format!("listing {}…", kind.plural), Role::Working)),
    }
}

/// Every row of every key, in key order; the ones whose name holds every name word.
fn rows<'a>(
    app: &'a AppState,
    keys: &'a [WatchKey],
    names: &[&str],
) -> Vec<(&'a WatchKey, &'a ObjectRow)> {
    let mut out = Vec::new();
    for key in keys {
        let Some(watched) = app.cluster.store.watched(key) else {
            continue;
        };
        let named = |row: &&ObjectRow| {
            names
                .iter()
                .all(|word| groove_types::fuzzy(&row.name, word))
        };
        out.extend(watched.rows.iter().filter(named).map(|row| (key, row)));
    }
    out
}

/// The rows in the order picked, under their columns: each as wide as dragged, else fitted.
fn listed(
    ctx: &mut Ctx,
    body: Rect,
    app: &AppState,
    (keys, mut rows): (&[WatchKey], Vec<(&WatchKey, &ObjectRow)>),
    (held, kind): (&ResourcesUi, &str),
) {
    let plan = plan(app, keys, &rows);
    let sorted = order::sorted(&mut rows, &plan, held.sort.as_ref());
    let named = usize::from(plan.clusters) + usize::from(plan.namespaces);
    let dragged = held.widths.get(kind);
    let columns: Vec<Column<'_, Target>> = plan
        .labels
        .iter()
        .zip(&plan.samples)
        .enumerate()
        .map(|(at, (label, sample))| {
            let width = match (dragged.and_then(|one| one.get(label)), at == named) {
                (Some(wide), _) => Width::Fixed(*wide),
                (None, true) => Width::Fill,
                (None, false) => Width::Fit(sample),
            };
            column(label, width)
        })
        .collect();
    let rows = rows.as_slice();
    let offset = held.scroll;
    let table = Table {
        columns: &columns,
        rows: Rows {
            first: ctx.tokens.row,
            under: 0.0,
            ruled: false,
        },
        count: rows.len(),
        offset,
        selected: None,
        sorted: Some(sorted.unwrap_or(Sorted::NONE)),
    };
    let extent = table.draw(ctx, body, |at| one(app, rows[at], &plan));
    ctx.app.hits.scrolls(Scroller::Resources, extent);
}

fn one<'a>(
    app: &AppState,
    (key, row): (&'a WatchKey, &'a ObjectRow),
    plan: &Plan,
) -> Shown<'a, Target> {
    let mut cells = Vec::new();
    if plan.clusters {
        let hue = app.config.cluster(&key.context).map(|one| one.hue);
        cells.push(Cell::small(
            key.context.as_str(),
            hue.map_or(Role::Faint, Role::Hue),
        ));
    }
    if plan.namespaces {
        cells.push(Cell::small(
            row.namespace.as_deref().unwrap_or_default(),
            Role::Muted,
        ));
    }
    let cell = |at: usize| row.cells.get(at).map(String::as_str).unwrap_or_default();
    let ended = plan.shown.iter().zip(&plan.reads).any(|(at, reads)| {
        *reads == Reads::Status && groove_types::status_health(cell(*at)) == Some(Health::Done)
    });
    for (place, (at, reads)) in plan.shown.iter().zip(&plan.reads).enumerate() {
        let text = cell(*at);
        let health = match reads {
            Reads::Status => groove_types::status_health(text),
            Reads::Ready if ended => Some(Health::Done),
            Reads::Ready => groove_types::ready_health(text),
            Reads::Plain => None,
        };
        let role = match (place, health) {
            (0, _) => Role::Text,
            (_, Some(health)) => colour(health),
            _ => Role::Muted,
        };
        cells.push(Cell::small(text, role));
    }
    Shown {
        cells,
        under: Vec::new(),
        target: Some(Target::ResourceRow(row.uid.clone())),
        acts: Vec::new(),
    }
}

fn column<'a>(label: &'a str, width: Width<'a>) -> Column<'a, Target> {
    Column {
        label,
        width,
        end: false,
        sort: Some(Target::ResourceSort(label.to_string())),
        edge: Some(Target::ResourceEdge(label.to_string())),
    }
}
