//! An Application's resources view: the filters, then its resources by kind, what needs attention first.

use std::collections::BTreeMap;

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};
use groove_types::{AppPart, AppResource, SyncedResource};
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, column_in};
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{Cell, Shown, Tab, Text};

use super::super::Filter;
use super::super::opened::Opened;
use super::grouped::{self, Columned};
use super::{destined, stages, syncing};
use crate::ctx::Ctx;
use crate::hit::Target;

const COLUMNS: [Columned; 5] = [
    ("SYNC", "✓"),
    ("NAME", "billing-eu-config-manager"),
    ("NAMESPACE", "billing-eu-staging"),
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
    (app, tab, part): (&AppState, &Opened, &AppPart),
    dragged: &BTreeMap<String, f32>,
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
        grouped::order(&mut group.rows, tab, |one, column| key(part, one, column));
    }
    let lines = lines((tab, part), &groups, filter);
    let count = lines.len();
    let context = destined::context(app, &tab.link.context, &part.destination);
    let link = |one: &AppResource| destined::link(app, context.as_deref(), one);
    grouped::draw(
        ctx,
        rest,
        (tab, dragged),
        (&COLUMNS, count),
        |at| match &lines[at] {
            Line::Row(one) => resource(part, one)
                .target(link(one).map(|link| Target::ResourceOpen(Box::new(link)))),
            line => shown(part, line),
        },
    );
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
    let mut room = strip.pad(Edges::across(ctx.tokens.sm, ctx.tokens.sm));
    for filter in Filter::OFFERED {
        let count = match filter {
            Filter::Syncing => stages::remaining(part),
            _ => part
                .resources
                .iter()
                .filter(|one| filter.holds(part, one))
                .count(),
        };
        let role = match filter {
            Filter::Attention => Role::Bad,
            Filter::OutOfSync => Role::Warn,
            Filter::Syncing => Role::Working,
            Filter::All => Role::Muted,
        };
        let said = format!("{} · {count}", filter.label());
        Tab::new(&said, Target::ArgoFilter(filter), filter == on)
            .glass()
            .text(Text::Small)
            .quiet(role)
            .left(ctx, &mut room, ctx.tokens.xs);
    }
}

fn groups(part: &AppPart, filter: Filter) -> Vec<Group<'_>> {
    let mut by: BTreeMap<&str, Vec<&AppResource>> = BTreeMap::new();
    for one in part.resources.iter().filter(|one| filter.holds(part, one)) {
        by.entry(&one.kind).or_default().push(one);
    }
    let first = |one: &&AppResource| (!part.attention(one), one.name.clone());
    let mut out: Vec<Group> = by
        .into_iter()
        .map(|(kind, mut rows)| {
            rows.sort_by_key(first);
            Group { kind, rows }
        })
        .collect();
    out.sort_by_key(|group| !group.rows.iter().any(|one| part.attention(one)));
    out
}

/// Each group's heading, then its rows unless folded; under all, a group with nothing to see starts folded.
fn lines<'a>(
    (tab, part): (&Opened, &AppPart),
    groups: &'a [Group<'a>],
    filter: Filter,
) -> Vec<Line<'a>> {
    let mut out = Vec::new();
    for group in groups {
        let quiet = !group.rows.iter().any(|one| part.attention(one));
        let shut = tab.folded(group.kind, filter == Filter::All && quiet);
        out.push(Line::Heading(group, shut));
        if !shut {
            out.extend(group.rows.iter().map(|one| Line::Row(one)));
        }
    }
    out
}

fn shown<'a>(part: &'a AppPart, line: &Line<'a>) -> Shown<'a, Target> {
    match line {
        Line::Heading(group, shut) => heading(part, group, *shut),
        Line::Row(one) => resource(part, one),
    }
}

/// `Deployments 4 1 out of sync · 1 failed`, after the fold caret.
fn heading<'a>(part: &AppPart, group: &Group<'a>, shut: bool) -> Shown<'a, Target> {
    let (said, role) = note(part, &group.rows);
    let cells = vec![
        Cell::small(group.kind, Role::Text),
        Cell::small(group.rows.len().to_string(), Role::Ghost),
        Cell::small(said, role),
    ];
    grouped::heading((group.kind, shut), cells)
}

/// How many are out of sync, wait to be pruned, and failed in the last sync.
fn note(part: &AppPart, rows: &[&AppResource]) -> (String, Role) {
    let count = |test: &dyn Fn(&AppResource) -> bool| rows.iter().filter(|one| test(one)).count();
    let failed = count(&|one| part.result(one).is_some_and(SyncedResource::broke));
    let counted = [
        (count(&AppResource::out_of_sync), "out of sync"),
        (count(&|one| one.prune), "to prune"),
        (failed, "failed"),
    ];
    let said: Vec<String> = counted
        .iter()
        .filter(|(count, _)| *count > 0)
        .map(|(count, word)| format!("{count} {word}"))
        .collect();
    let role = match () {
        _ if failed > 0 => Role::Bad,
        _ if !said.is_empty() => Role::Warn,
        _ => Role::Ghost,
    };
    (said.join(" · "), role)
}

/// What the resource says: waiting to be pruned, else its last sync's message.
fn status<'a>(part: &'a AppPart, one: &AppResource) -> (&'a str, Role) {
    let result = part.result(one);
    match result {
        _ if one.prune => ("to prune", Role::Warn),
        Some(done) if done.broke() => (done.message.as_str(), Role::Bad),
        Some(done) => (done.message.as_str(), Role::Muted),
        None => ("", Role::Muted),
    }
}

fn resource<'a>(part: &'a AppPart, one: &'a AppResource) -> Shown<'a, Target> {
    let (sync, synced_role) = synced(one);
    let (message, role) = status(part, one);
    Shown::row(vec![
        Cell::small(sync, synced_role),
        Cell::body(one.name.as_str(), Role::Text),
        Cell::small(one.namespace.as_deref().unwrap_or_default(), Role::Faint),
        Cell::small(one.wave.to_string(), Role::Ghost),
        Cell::body(message, role),
    ])
}

/// What a column sorts a resource by; a wave by its number.
fn key(part: &AppPart, one: &AppResource, column: &str) -> String {
    match column {
        "SYNC" => one.sync.clone(),
        "NAMESPACE" => one.namespace.clone().unwrap_or_default(),
        "WAVE" => format!("{:020}", i128::from(one.wave) - i128::from(i64::MIN)),
        "STATUS" => status(part, one).0.to_string(),
        _ => one.name.clone(),
    }
}

fn synced(one: &AppResource) -> (&'static str, Role) {
    if one.hook {
        return ("hook", Role::Ghost);
    }
    if !one.out_of_sync() {
        return ("✓", Role::Ok);
    }
    match one.sync.as_str() {
        "OutOfSync" => ("◐", Role::Warn),
        _ => ("?", Role::Ghost),
    }
}
