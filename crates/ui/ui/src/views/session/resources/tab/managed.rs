//! An Application's resources view: the filters, then its resources by kind, what is notable first.

use std::collections::BTreeMap;

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};
use groove_types::{AppPart, AppResource, ResourceHealth, SyncedResource, health_of};
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, column_in};
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{Cell, Shown, Tab, Text};

use groove_controllers::cluster_service::Treed;

use super::super::health::status_colour;
use super::super::opened::{Link, Opened};
use super::super::{ArgoUi, Filter};
use super::grouped::{self, Columned};
use super::{destined, stages, syncing};
use crate::ctx::Ctx;
use crate::hit::Target;

const COLUMNS: [Columned; 6] = [
    ("SYNC", "✓"),
    ("NAME", "billing-eu-config-manager"),
    ("NAMESPACE", "billing-eu-staging"),
    ("HEALTH", "Progressing"),
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
    let read = app.cluster.store.trees.get(&tab.link.pod());
    let health = match read {
        Some(Ok(all)) => all.as_slice(),
        _ => &[],
    };
    let filter = tab.argo.filter(part, health);
    filters(ctx, strip, ((part, health), read), filter);
    if filter == Filter::Syncing {
        return syncing::draw(ctx, rest, (tab, part), dragged);
    }
    let rest = super::found::bar(ctx, rest, tab);
    let mut groups = groups((part, health), (filter, &tab.argo));
    for group in &mut groups {
        grouped::order(&mut group.rows, tab, |one, column| {
            key((part, health), one, column)
        });
    }
    let lines = lines((tab, (part, health)), &groups, filter);
    let count = lines.len();
    let context = destined::context(app, &tab.link.context, &part.destination);
    let link = |one: &AppResource| destined::link(app, context.as_deref(), one);
    grouped::draw(
        ctx,
        rest,
        (tab, dragged),
        (&COLUMNS, count),
        |at| match &lines[at] {
            Line::Row(one) => resource((part, health), one).target(opens(filter, one, link(one))),
            line => shown((part, health), line),
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

/// Out of sync, a row opens its drift in the diff; else its object, when its context is known.
fn opens(filter: Filter, one: &AppResource, link: Option<Link>) -> Option<Target> {
    let link = link.map(Box::new);
    if filter != Filter::OutOfSync {
        return link.map(Target::ResourceOpen);
    }
    let path = groove_types::drift_path(&one.kind, one.namespace.as_deref(), &one.name);
    Some(Target::ResourceDrift(path, link))
}

/// A tab a filter, each in its colour and with its count; at the right, a refused health read.
fn filters(ctx: &mut Ctx, strip: Rect, (held, read): (Held, Option<&Treed>), on: Filter) {
    let (part, health) = held;
    let mut room = strip.pad(Edges::across(ctx.tokens.sm, ctx.tokens.sm));
    if let Some((said, role)) = health_read(read) {
        Label::new(&said, ctx.styles.small(role)).right(ctx, &mut room, ctx.tokens.sm);
    }
    for filter in Filter::OFFERED {
        let count = match filter {
            Filter::Syncing => stages::remaining(part),
            _ => part
                .resources
                .iter()
                .filter(|one| filter.holds(health, one))
                .count(),
        };
        let role = match filter {
            Filter::Degraded => Role::Bad,
            Filter::OutOfSync => Role::Warn,
            Filter::Syncing => Role::Working,
            Filter::All => Role::Muted,
        };
        let said = format!("{} · {count}", filter.label());
        Tab::new(&said, Target::ArgoFilter(filter), filter == on)
            .text(Text::Small)
            .quiet(role)
            .left(ctx, &mut room, ctx.tokens.xs);
    }
}

/// The first line of what argocd said when it refused the health read.
fn health_read(read: Option<&Treed>) -> Option<(String, Role)> {
    let Err(why) = read? else {
        return None;
    };
    let first = why.lines().next().unwrap_or_default();
    Some((format!("no health: {first}"), Role::Warn))
}

fn groups<'a>((part, health): Held<'a>, (filter, argo): (Filter, &ArgoUi)) -> Vec<Group<'a>> {
    let mut by: BTreeMap<&str, Vec<&AppResource>> = BTreeMap::new();
    let kept = |one: &&AppResource| filter.holds(health, one) && argo.found(one);
    for one in part.resources.iter().filter(kept) {
        by.entry(&one.kind).or_default().push(one);
    }
    let first = |one: &&AppResource| (!part.notable(one, health), one.name.clone());
    let mut out: Vec<Group> = by
        .into_iter()
        .map(|(kind, mut rows)| {
            rows.sort_by_key(first);
            Group { kind, rows }
        })
        .collect();
    out.sort_by_key(|group| !group.rows.iter().any(|one| part.notable(one, health)));
    out
}

/// Each group's heading, then its rows unless folded; under all, a group with nothing to see starts folded.
fn lines<'a>(
    (tab, (part, health)): (&Opened, Held),
    groups: &'a [Group<'a>],
    filter: Filter,
) -> Vec<Line<'a>> {
    let mut out = Vec::new();
    for group in groups {
        let quiet = !group.rows.iter().any(|one| part.notable(one, health));
        let shut = tab.folded(group.kind, filter == Filter::All && quiet);
        out.push(Line::Heading(group, shut));
        if !shut {
            out.extend(group.rows.iter().map(|one| Line::Row(one)));
        }
    }
    out
}

fn shown<'a>(held: Held<'a>, line: &Line<'a>) -> Shown<'a, Target> {
    match line {
        Line::Heading(group, shut) => heading(held, group, *shut),
        Line::Row(one) => resource(held, one),
    }
}

/// `Deployments 4 1 out of sync · 1 failed · 1 degraded`, after the fold caret.
fn heading<'a>(held: Held<'a>, group: &Group<'a>, shut: bool) -> Shown<'a, Target> {
    let (said, role) = note(held, &group.rows);
    let cells = vec![
        Cell::small(group.kind, Role::Text),
        Cell::small(group.rows.len().to_string(), Role::Ghost),
        Cell::small(said, role),
    ];
    grouped::heading((group.kind, shut), cells)
}

/// How many are out of sync, wait to be pruned, failed in the last sync, are degraded or progress.
fn note((part, health): Held, rows: &[&AppResource]) -> (String, Role) {
    let count = |test: &dyn Fn(&AppResource) -> bool| rows.iter().filter(|one| test(one)).count();
    let failed = count(&|one| part.result(one).is_some_and(SyncedResource::broke));
    let health = |word: &'static str| move |one: &AppResource| health_of(health, one) == word;
    let degraded = count(&health("Degraded"));
    let counted = [
        (count(&AppResource::out_of_sync), "out of sync"),
        (count(&|one| one.prune), "to prune"),
        (failed, "failed"),
        (degraded, "degraded"),
        (count(&health("Progressing")), "progressing"),
    ];
    let said: Vec<String> = counted
        .iter()
        .filter(|(count, _)| *count > 0)
        .map(|(count, word)| format!("{count} {word}"))
        .collect();
    let role = match () {
        _ if failed + degraded > 0 => Role::Bad,
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

/// The Application, and each of its resources' health as last read.
type Held<'a> = (&'a AppPart, &'a [ResourceHealth]);

fn resource<'a>((part, health): Held<'a>, one: &'a AppResource) -> Shown<'a, Target> {
    let (sync, synced_role) = synced(one);
    let (message, role) = status(part, one);
    let said = health_of(health, one);
    let health = match said.is_empty() {
        true => Cell::small("–", Role::Ghost),
        false => Cell::small(said, status_colour(said)),
    };
    Shown::row(vec![
        Cell::small(sync, synced_role),
        Cell::body(one.name.as_str(), Role::Text),
        Cell::small(one.namespace.as_deref().unwrap_or_default(), Role::Faint),
        health,
        Cell::small(one.wave.to_string(), Role::Ghost),
        Cell::body(message, role),
    ])
}

/// What a column sorts a resource by; a wave by its number.
fn key((part, health): Held, one: &AppResource, column: &str) -> String {
    match column {
        "SYNC" => one.sync.clone(),
        "NAMESPACE" => one.namespace.clone().unwrap_or_default(),
        "HEALTH" => health_of(health, one).to_string(),
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
