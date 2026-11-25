//! The Resources tab's panel: the list tab, titled by its kind and count, over the rows.

use groove_controllers::AppState;
use groove_controllers::session_service::Open;
use groove_gfx::{Edges, Rect};
use groove_types::{KubeKind, ObjectRow, WatchKey};
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, column_in};
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{
    Cell, Column, Laid, Rows, Search, Shown, Sorted, Tab, Table, Width, glass_field,
};

mod cells;
mod order;

pub use order::Kept;
pub(super) mod plan;

use self::cells::{column, one};
use self::plan::plan;
use super::Written;
use super::scope::{self, ResourcesUi};
use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::{Scroller, Target};

/// The rows' uids, in the order the list draws them.
pub fn uids(app: &AppState, ui: &Ui) -> Vec<String> {
    let Some(open) = app.session.selected() else {
        return Vec::new();
    };
    let held = &ui.session.resources;
    let keys = scope::keys(app, open, held);
    let picked = (keys.as_slice(), held.search.text(), held.sort.as_ref());
    let order = held
        .kept
        .of(app, picked, || order::ordered(app, &keys, held));
    let rows = resolved(app, &keys, &order);
    rows.into_iter().map(|(_, row)| row.uid.clone()).collect()
}

pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui, body: Rect) {
    let Some(open) = app.session.selected() else {
        return;
    };
    let held = &ui.session.resources;
    let band = Spec::default().height(ctx.tokens.row + ctx.tokens.sm);
    let [strip, mut body] = column_in(body, [band, Spec::fill()]);
    let keys = scope::keys(app, open, held);
    let picked = (keys.as_slice(), held.search.text(), held.sort.as_ref());
    let order = held
        .kept
        .of(app, picked, || order::ordered(app, &keys, held));
    let rows = resolved(app, &keys, &order);
    let kind = scope::kind(app, open, held);
    title(ctx, strip, (kind.as_ref(), rows.len()), held);
    if let Some(tab) = held.tab() {
        return super::tab::draw(ctx, body, app, (ui, tab));
    }
    if held.finding || !held.search.is_empty() {
        let [bar, rest] = column_in(body, [Spec::default().height(ctx.tokens.row), Spec::fill()]);
        found(ctx, bar, held);
        body = rest;
    }
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
    let asked = match ui.losing() {
        Some(crate::Losing::Row(written)) => Some(&**written),
        _ => None,
    };
    listed(ctx, body, app, (&keys, rows), (held, &kind, asked));
}

/// The find bar over the rows: what it narrows by, typed.
fn found(ctx: &mut Ctx, line: Rect, held: &ResourcesUi) {
    let field = glass_field(ctx, line);
    let search = Search::new(&held.search, Target::ResourceSearch, held.typing);
    search
        .hint("name, or label=value")
        .code()
        .faint(Role::Faint)
        .draw(ctx, field);
}

/// The list's own tab, its kind and how many rows it holds, then a tab for each object open.
fn title(
    ctx: &mut Ctx,
    strip: Rect,
    (kind, count): (Option<&KubeKind>, usize),
    held: &ResourcesUi,
) {
    let mut room = strip.pad(Edges::across(ctx.tokens.sm, ctx.tokens.md));
    let xs = ctx.tokens.xs;
    let named = kind.map(|kind| format!("{} \u{b7} {count}", plural(kind)));
    let mut tabs = Vec::new();
    if let Some(named) = &named {
        tabs.push(Tab::new(
            named,
            Target::ResourceList,
            held.showing.is_none(),
        ));
    }
    for (at, one) in held.opened.iter().enumerate() {
        let shown = held.showing == Some(at);
        let tab = Tab::new(&one.link.name, Target::ResourceTab(at), shown).keep_case();
        tabs.push(tab.close(Mark::Close, Target::ResourceClose(at)));
    }
    if !tabs.is_empty() {
        let side = groove_ui_kit::widgets::Side::Left;
        groove_ui_kit::widgets::tab_group(ctx, &mut room, tabs, (side, xs));
    }
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
    if scope::held(open, held).is_empty() {
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

/// The rows an order names, as they stand.
fn resolved<'a>(
    app: &'a AppState,
    keys: &'a [WatchKey],
    order: &[(usize, usize)],
) -> Vec<(&'a WatchKey, &'a ObjectRow)> {
    let watched: Vec<_> = keys
        .iter()
        .map(|key| app.cluster.store.watched(key))
        .collect();
    let at = |(place, at): &(usize, usize)| {
        let row = watched.get(*place).copied().flatten()?.rows.get(*at)?;
        Some((&keys[*place], row))
    };
    order.iter().filter_map(at).collect()
}

/// The rows in the order picked, under their columns: each as wide as dragged, else fitted.
fn listed(
    ctx: &mut Ctx,
    body: Rect,
    app: &AppState,
    (keys, rows): (&[WatchKey], Vec<(&WatchKey, &ObjectRow)>),
    (held, kind, asked): (&ResourcesUi, &str, Option<&Written>),
) {
    let plan = plan(app, keys, &rows);
    let sorted = order::mark(&plan, held.sort.as_ref());
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
                (None, false) => Width::FitBody(sample),
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
            carded: false,
            gridded: true,
        },
        count: rows.len(),
        offset,
        selected: None,
        sorted: Some(sorted.unwrap_or(Sorted::NONE)),
    };
    let (_, names) = scope::search(held);
    let body = body.pad(Edges::across(ctx.tokens.sm, ctx.tokens.sm));
    let at = asked.and_then(|one| rows.iter().position(|(_, row)| row.uid == one.uid));
    let laid = table.draw(ctx, body, |row| {
        shown(app, rows[row], (&plan, &names), Some(row) == at)
    });
    asking(ctx, &laid, at.zip(asked));
    ctx.app.hits.scrolls(Scroller::Resources, laid.extent);
}

/// The row's cells, or none while a question stands in its place.
fn shown<'a>(
    app: &AppState,
    row: (&'a WatchKey, &'a ObjectRow),
    read: (&self::plan::Plan, &[&str]),
    asked: bool,
) -> Shown<'a, Target> {
    match asked {
        true => Shown::row(Vec::<Cell>::new()),
        false => one(app, row, read),
    }
}

/// The write asked across the row's whole width, a scale's count stepped before the answers.
fn asking(ctx: &mut Ctx, laid: &Laid, asked: Option<(usize, &Written)>) {
    let Some((at, written)) = asked else {
        return;
    };
    let (line, room) = (laid.across(at), laid.body);
    if line.y < room.y || line.y + line.h > room.y + room.h {
        return;
    }
    super::written::ask(ctx, line, written);
}
