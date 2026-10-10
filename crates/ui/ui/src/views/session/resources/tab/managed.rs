//! An Application's resources view: the filters, then its resources by kind, what needs attention first.

use std::collections::BTreeMap;

use groove_gfx::{Edges, Rect};
use groove_types::{AppPart, AppResource};
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, column_in};
use groove_ui_kit::shape::hairline;
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{Cell, Shown, Tab, Text};

use super::super::Filter;
use super::super::opened::Opened;
use super::grouped::{self, Columned};
use super::{stages, syncing};
use crate::ctx::Ctx;
use crate::hit::Target;

const COLUMNS: [Columned; 6] = [
    ("SYNC", "✓"),
    ("HEALTH", "♥"),
    ("NAME", "pythie-cayzn-config-manager"),
    ("NAMESPACE", "pythie-cayzn-staging"),
    ("WAVE", "-10"),
    ("STATUS", ""),
];

/// The resources of one kind that the filter keeps.
struct Group<'a> {
    kind: &'a str,
    rows: Vec<&'a AppResource>,
}

enum Line<'a> {
    Heading(&'a Group<'a>, bool),
    Row(&'a AppResource),
}

pub(super) fn draw(
    ctx: &mut Ctx,
    body: Rect,
    (tab, part): (&Opened, &AppPart),
    dragged: Option<&BTreeMap<String, f32>>,
) {
    let tall = Spec::default().height(ctx.tokens.row + ctx.tokens.sm * 2.0);
    let [strip, rest] = column_in(body, [tall, Spec::fill()]);
    let filter = tab.argo.filter(part);
    filters(ctx, strip, part, filter);
    if filter == Filter::Syncing {
        return syncing::draw(ctx, rest, (tab, part), dragged);
    }
    let mut groups = groups(part, filter);
    for group in &mut groups {
        grouped::order(&mut group.rows, tab, key);
    }
    let lines = lines(tab, &groups, filter);
    let count = lines.len();
    grouped::draw(ctx, rest, (tab, dragged), (&COLUMNS, count), |at| {
        shown(&lines[at])
    });
    if count == 0 {
        let row = Spec::default().height(ctx.tokens.row);
        let [_, line, _] = column_in(rest, [row, row, Spec::fill()]);
        let line = line.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
        let said = format!("no resource {}", filter.label());
        Label::new(&said, ctx.styles.small(Role::Faint)).draw(ctx, line);
    }
}

/// A tab a filter, each in its colour and with its count.
fn filters(ctx: &mut Ctx, strip: Rect, part: &AppPart, on: Filter) {
    hairline(ctx, strip, ctx.styles.line());
    let mut room = strip;
    for filter in Filter::OFFERED {
        let count = match filter {
            Filter::Syncing => stages::remaining(part),
            _ => part
                .resources
                .iter()
                .filter(|one| filter.holds(one))
                .count(),
        };
        let role = match filter {
            Filter::Attention | Filter::Unhealthy => Role::Bad,
            Filter::OutOfSync => Role::Warn,
            Filter::Syncing => Role::Working,
            Filter::All => Role::Muted,
        };
        let said = format!("{} · {count}", filter.label());
        Tab::new(&said, Target::ArgoFilter(filter), filter == on)
            .text(Text::Small)
            .quiet(role)
            .left(ctx, &mut room, 0.0);
    }
}

fn groups(part: &AppPart, filter: Filter) -> Vec<Group<'_>> {
    let mut by: BTreeMap<&str, Vec<&AppResource>> = BTreeMap::new();
    for one in part.resources.iter().filter(|one| filter.holds(one)) {
        by.entry(&one.kind).or_default().push(one);
    }
    let mut out: Vec<Group> = by
        .into_iter()
        .map(|(kind, mut rows)| {
            rows.sort_by(|a, b| (!a.attention(), &a.name).cmp(&(!b.attention(), &b.name)));
            Group { kind, rows }
        })
        .collect();
    out.sort_by_key(|group| !group.rows.iter().any(|one| one.attention()));
    out
}

/// Each group's heading, then its rows unless folded; under all, a group with nothing to see starts folded.
fn lines<'a>(tab: &Opened, groups: &'a [Group<'a>], filter: Filter) -> Vec<Line<'a>> {
    let mut out = Vec::new();
    for group in groups {
        let quiet = !group.rows.iter().any(|one| one.attention());
        let shut = tab.argo.shut(group.kind, filter == Filter::All && quiet);
        out.push(Line::Heading(group, shut));
        if !shut {
            out.extend(group.rows.iter().map(|one| Line::Row(one)));
        }
    }
    out
}

fn shown<'a>(line: &Line<'a>) -> Shown<'a, Target> {
    match line {
        Line::Heading(group, shut) => heading(group, *shut),
        Line::Row(one) => resource(one),
    }
}

/// `▾ Deployments 4 1 out of sync · 1 progressing`.
fn heading<'a>(group: &Group<'a>, shut: bool) -> Shown<'a, Target> {
    let mark = if shut { "▸" } else { "▾" };
    let (said, role) = note(&group.rows);
    let cells = vec![
        Cell::small(mark, Role::Ghost),
        Cell::small(group.kind, Role::Text),
        Cell::small(group.rows.len().to_string(), Role::Ghost),
        Cell::small(said, role),
    ];
    grouped::heading(group.kind, cells)
}

/// How many are out of sync, then how many stand in each health but healthy.
fn note(rows: &[&AppResource]) -> (String, Role) {
    let drifted = rows.iter().filter(|one| one.out_of_sync()).count();
    let mut said = Vec::new();
    if drifted > 0 {
        said.push(format!("{drifted} out of sync"));
    }
    for health in ["Degraded", "Missing", "Progressing", "Suspended", "Unknown"] {
        let count = rows.iter().filter(|one| one.health == health).count();
        if count > 0 {
            said.push(format!("{count} {}", health.to_lowercase()));
        }
    }
    let has = |health: &str| rows.iter().any(|one| one.health == health);
    let role = match () {
        _ if has("Degraded") || has("Missing") => Role::Bad,
        _ if drifted > 0 => Role::Warn,
        _ if has("Progressing") => Role::Working,
        _ => Role::Ghost,
    };
    (said.join(" · "), role)
}

fn resource(one: &AppResource) -> Shown<'_, Target> {
    let (sync, synced_role) = synced(one);
    let (health, health_role) = healthy(&one.health);
    let (message, role) = match (one.prune, one.health.as_str()) {
        (true, _) => ("to prune", Role::Warn),
        (_, "Degraded" | "Missing") => (one.message.as_str(), Role::Bad),
        _ => (one.message.as_str(), Role::Muted),
    };
    grouped::row(vec![
        Cell::small(sync, synced_role),
        Cell::small(health, health_role),
        Cell::small(one.name.as_str(), Role::Text),
        Cell::small(one.namespace.as_deref().unwrap_or_default(), Role::Faint),
        Cell::small(one.wave.to_string(), Role::Ghost),
        Cell::small(message, role),
    ])
}

/// What a column sorts a resource by; a wave by its number.
fn key(one: &&AppResource, column: &str) -> String {
    match column {
        "SYNC" => one.sync.clone(),
        "HEALTH" => one.health.clone(),
        "NAMESPACE" => one.namespace.clone().unwrap_or_default(),
        "WAVE" => format!("{:020}", i128::from(one.wave) - i128::from(i64::MIN)),
        "STATUS" => one.message.clone(),
        _ => one.name.clone(),
    }
}

fn synced(one: &AppResource) -> (&'static str, Role) {
    if one.hook {
        return ("hook", Role::Ghost);
    }
    match one.sync.as_str() {
        "Synced" => ("✓", Role::Ok),
        "OutOfSync" => ("◐", Role::Warn),
        _ => ("?", Role::Ghost),
    }
}

pub(super) fn healthy(health: &str) -> (&'static str, Role) {
    match health {
        "Healthy" => ("♥", Role::Ok),
        "Progressing" => ("◌", Role::Working),
        "Degraded" => ("✗", Role::Bad),
        "Missing" => ("◇", Role::Bad),
        "Suspended" => ("‖", Role::Muted),
        "" => ("", Role::Ghost),
        _ => ("?", Role::Ghost),
    }
}
