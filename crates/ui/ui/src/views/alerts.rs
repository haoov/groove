//! A maintenance task's alerts: a window over the agent, folding to its header in the bar above it.

mod table;

use groove_controllers::AppState;
use groove_controllers::session_service::Open;
use groove_gfx::{Edges, Rect};
use groove_types::{Alert, AlertChild};
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, column_in};
use groove_ui_kit::shape::band;
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{Column, Rows, Table, Width, flat_row, panel_head, solid_pane};

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::{Scroller, Target};

/// What the alerts' window remembers between frames.
#[derive(Debug, Clone, PartialEq)]
pub struct AlertsUi {
    /// The selected session is a maintenance task: the window stands.
    pub shown: bool,
    pub open: bool,
    pub scroll: f32,
    /// The row picked, by its alert's key.
    pub selected: Option<String>,
    /// The column the rows are ordered by, and whether downward.
    pub sort: Option<(String, bool)>,
    /// Each column's width as the user dragged it.
    pub widths: std::collections::BTreeMap<String, f32>,
}

impl Default for AlertsUi {
    fn default() -> Self {
        Self {
            shown: false,
            open: true,
            scroll: 0.0,
            selected: None,
            sort: None,
            widths: Default::default(),
        }
    }
}

/// One row: a firing alert, or one that resolved while its child is still on the rail.
pub(super) enum Row<'a> {
    Firing(&'a Alert, Option<&'a AlertChild>),
    Resolved(&'a AlertChild),
}

pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui) {
    let window = ctx.app.layout.alerts;
    let Some(open) = app.session.selected().filter(|_| !window.is_empty()) else {
        return;
    };
    solid_pane(ctx, window);
    let [head, body] = column_in(
        window,
        [Spec::default().height(ctx.tokens.bar), Spec::fill()],
    );
    panel_head(ctx, head, !ui.alerts.open);
    heading(ctx, head, app, ui);
    if ui.alerts.open {
        rows(ctx, body, app, ui, open);
    }
}

/// The word that folds the window, then how many fire at each rank, or why the read failed.
fn heading(ctx: &mut Ctx, line: Rect, app: &AppState, ui: &Ui) {
    let row = flat_row(ctx, line, Target::AlertsFold, false);
    let tall = ctx.tokens.row;
    let mut room = band(row, (row.h - tall) / 2.0, tall);
    room = room.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
    let sm = ctx.tokens.sm;
    let [critical, on_call, warning, _] = app.alert.counts();
    if let Some(why) = &app.alert.failed {
        Label::new(why, ctx.styles.small(Role::Bad)).right(ctx, &mut room, sm);
    }
    for (count, word, role) in [
        (warning, "warning", Role::Warn),
        (on_call, "on call", Role::Attention),
        (critical, "critical", Role::Bad),
    ] {
        if count > 0 {
            let said = format!("{count} {word}");
            Label::new(&said, ctx.styles.small(role)).right(ctx, &mut room, sm);
        }
    }
    groove_ui_kit::widgets::Heading::new("alerts")
        .count(app.alert.firing.len())
        .fold(ui.alerts.open)
        .within(ctx, row, room);
}

/// The firing alerts, then the resolved ones whose child is on the rail, on the kit's grid.
fn rows(ctx: &mut Ctx, body: Rect, app: &AppState, ui: &Ui, open: &Open) {
    let (sorted, all) = table::ordered(app, gathered(app, open), ui.alerts.sort.as_ref());
    let columns: Vec<Column<'_, Target>> = COLUMNS
        .iter()
        .map(|(label, width)| {
            let width = match ui.alerts.widths.get(*label) {
                Some(wide) => Width::Fixed(*wide),
                None => *width,
            };
            Column {
                end: *label == "age",
                sort: Some(Target::AlertColumn(label.to_string())),
                edge: Some(Target::AlertEdge(label.to_string())),
                ..Column::plain(label, width)
            }
        })
        .collect();
    let table = Table {
        columns: &columns,
        rows: Rows {
            first: ctx.tokens.row,
            under: 0.0,
            ruled: false,
            carded: false,
            gridded: true,
        },
        count: all.len(),
        offset: ui.alerts.scroll,
        selected: all
            .iter()
            .position(|row| Some(row.key()) == ui.alerts.selected.as_ref()),
        sorted: Some(sorted),
    };
    let now = ctx.now;
    let body = body.pad(Edges::across(ctx.tokens.sm, ctx.tokens.sm));
    let laid = table.draw(ctx, body, |at| table::shown(app, &all[at], now));
    ctx.app.hits.scrolls(Scroller::Alerts, laid.extent);
}

/// The firing alerts with their child, then the resolved ones whose child is on the rail.
fn gathered<'a>(app: &'a AppState, open: &'a Open) -> Vec<Row<'a>> {
    let child = |key: &str| open.alerted.iter().find(|one| one.dedup_key == key);
    let firing = app.alert.firing.iter();
    let mut all: Vec<Row> = firing
        .map(|one| Row::Firing(one, child(&one.dedup_key)))
        .collect();
    let resolved = open.alerted.iter().filter(|one| {
        app.alert.find(&one.dedup_key).is_none() && app.session.get(&one.session).is_some()
    });
    all.extend(resolved.map(Row::Resolved));
    all
}

/// The alerts' keys in the order the rows stand, for the keys to step through.
pub fn listed(app: &AppState, ui: &Ui) -> Vec<String> {
    let Some(open) = app.session.selected() else {
        return Vec::new();
    };
    let (_, all) = table::ordered(app, gathered(app, open), ui.alerts.sort.as_ref());
    all.iter().map(|row| row.key().clone()).collect()
}

impl Row<'_> {
    pub(super) fn key(&self) -> &String {
        match self {
            Row::Firing(one, _) => &one.dedup_key,
            Row::Resolved(one) => &one.dedup_key,
        }
    }
}

/// Every column: its label, and its width until the user drags it.
pub(super) const COLUMNS: [(&str, Width<'static>); 8] = [
    ("priority", Width::Fit("Critical")),
    ("alert", Width::Fill),
    ("environment", Width::Fit("Production")),
    ("cluster", Width::Fit("ovh-main-gra11-prod")),
    ("namespace", Width::Fit("cert-manager")),
    ("age", Width::Fit("12mo")),
    ("child", Width::Fit("working")),
    ("", Width::Fit("Investigate")),
];
