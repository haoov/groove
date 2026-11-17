//! A maintenance task's alerts: a window over the agent, folding to its header in the bar above it.

use groove_controllers::AppState;
use groove_controllers::session_service::Open;
use groove_gfx::{Edges, Rect};
use groove_types::{Alert, AlertChild};
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, column_in};
use groove_ui_kit::shape::band;
use groove_ui_kit::text::{Label, ago};
use groove_ui_kit::widgets::{flat_row, panel_head, solid_pane};

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::{Scroller, Target};
use crate::offsets::listed;

/// What the alerts' window remembers between frames.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AlertsUi {
    /// The selected session is a maintenance task: the window stands.
    pub shown: bool,
    pub open: bool,
    pub scroll: f32,
}

impl Default for AlertsUi {
    fn default() -> Self {
        Self {
            shown: false,
            open: true,
            scroll: 0.0,
        }
    }
}

/// One row: a firing alert, or one that resolved while its child is still on the rail.
enum Row<'a> {
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
        rows(ctx, body.pad(Edges::xy(0.0, ctx.tokens.xs)), app, ui, open);
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

fn rows(ctx: &mut Ctx, body: Rect, app: &AppState, ui: &Ui, open: &Open) {
    let child = |key: &str| open.alerted.iter().find(|one| one.dedup_key == key);
    let mut all: Vec<Row> = app
        .alert
        .firing
        .iter()
        .map(|one| Row::Firing(one, child(&one.dedup_key)))
        .collect();
    let resolved = open.alerted.iter().filter(|one| {
        app.alert.find(&one.dedup_key).is_none() && app.session.get(&one.session).is_some()
    });
    all.extend(resolved.map(Row::Resolved));
    let tall = ctx.tokens.row + ctx.tokens.xs;
    let at = (Scroller::Alerts, ui.alerts.scroll);
    listed(
        ctx,
        body,
        at,
        &all,
        |_| tall,
        |ctx, rect, row| drawn(ctx, app, rect, row),
    );
}

/// The rank's mark and the title; where it fires, how long, and its child or the way to one.
fn drawn(ctx: &mut Ctx, app: &AppState, rect: Rect, row: &Row) {
    let (key, title, place) = match row {
        Row::Firing(one, _) => (
            &one.dedup_key,
            &one.title,
            scope(&one.cluster, &one.namespace),
        ),
        Row::Resolved(one) => (
            &one.dedup_key,
            &one.title,
            scope(&one.cluster, &one.namespace),
        ),
    };
    let line = flat_row(ctx, rect, Target::Investigate(key.clone()), false);
    let (sm, md) = (ctx.tokens.sm, ctx.tokens.md);
    let mut room = line.pad(Edges::across(md, md));
    let (state, role) = match row {
        Row::Firing(_, Some(child)) | Row::Resolved(child) => child_state(app, child),
        Row::Firing(_, None) => ("investigate".to_string(), Role::Accent),
    };
    Label::new(&state, ctx.styles.small(role)).right(ctx, &mut room, md);
    let resolved = matches!(row, Row::Resolved(_));
    if let Row::Firing(one, _) = row {
        let age = ago(one.created_at.age_at(ctx.now));
        Label::new(&age, ctx.styles.small(Role::Ghost)).right(ctx, &mut room, md);
    }
    Label::new(&place, ctx.styles.code(Role::Muted)).right(ctx, &mut room, md);
    let (mark, text) = match row {
        Row::Firing(one, _) => (rank_role(one), Role::Text),
        Row::Resolved(_) => (Role::Ghost, Role::Ghost),
    };
    Label::new(if resolved { "○" } else { "●" }, ctx.styles.small(mark)).left(ctx, &mut room, sm);
    Label::new(title, ctx.styles.body(text)).draw(ctx, room);
}

fn scope(cluster: &Option<String>, namespace: &Option<String>) -> String {
    match (cluster, namespace) {
        (Some(cluster), Some(namespace)) => format!("{cluster}/{namespace}"),
        (Some(cluster), None) => cluster.clone(),
        _ => String::new(),
    }
}

fn rank_role(one: &Alert) -> Role {
    match one.rank() {
        0 => Role::Bad,
        1 => Role::Attention,
        2 => Role::Warn,
        _ => Role::Faint,
    }
}

/// What the child's agent does, or `closed` once it left the rail.
fn child_state(app: &AppState, child: &AlertChild) -> (String, Role) {
    match app.session.get(&child.session) {
        Some(open) => crate::views::rail::state_of(app, open),
        None => ("closed".to_string(), Role::Ghost),
    }
}
