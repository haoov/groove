//! The described view: one column of sections, each under a heading that folds it.

use std::sync::Arc;

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};
use groove_types::{Described, PodPart, Usage};
use groove_ui_kit::base::style::Role;
use groove_ui_kit::widgets::scrolled;

use super::super::opened::{Opened, Section, events_key};
use super::card;
use super::relations::{self, Columns};
use super::sections::{self, ACROSS, Fact};
use crate::ctx::Ctx;
use crate::hit::Scroller;

/// One block of the column, stacked from the top.
enum Block<'a> {
    Heading(Section, String),
    Conditions(&'a PodPart),
    Summary(Vec<Fact>),
    Relations(Columns),
    Card(&'a PodPart),
    Pair(&'a (String, String)),
    Event(&'a groove_types::EventRow),
    Gap,
}

pub(super) fn draw(
    ctx: &mut Ctx,
    body: Rect,
    app: &AppState,
    (tab, object): (&Opened, &Described),
) {
    let events = events(app, tab, object);
    let blocks = blocks(app, (tab, object), &events, ctx.now);
    let room = body.pad(Edges::all(ctx.tokens.md));
    let usage = usage(app, tab);
    let keys = key_width(ctx, object);
    let heights: Vec<f32> = blocks.iter().map(|one| height(ctx, tab, one)).collect();
    let indexed: Vec<(usize, &Block)> = blocks.iter().enumerate().collect();
    let extent = scrolled(
        ctx,
        room,
        tab.scroll,
        &indexed,
        |(at, _)| heights[*at],
        |ctx, rect, (_, block)| match block {
            Block::Heading(section, count) => {
                sections::heading(ctx, rect, (*section, tab.shut.contains(section)), count)
            }
            Block::Conditions(pod) => sections::conditions(ctx, rect, pod),
            Block::Summary(facts) => sections::grid(ctx, rect, facts),
            Block::Relations(columns) => relations::draw(ctx, rect, columns),
            Block::Card(pod) => card::draw(ctx, rect, (tab, pod), usage.as_ref()),
            Block::Pair(pair) => sections::pair(ctx, rect, pair, keys),
            Block::Event(row) => sections::event(ctx, rect, row),
            Block::Gap => {}
        },
    );
    ctx.app.hits.scrolls(Scroller::Resources, extent);
}

fn height(ctx: &Ctx, tab: &Opened, block: &Block) -> f32 {
    let row = ctx.tokens.row;
    match block {
        Block::Summary(facts) => row * facts.len().div_ceil(ACROSS) as f32,
        Block::Relations(columns) => row * columns.rows() as f32,
        Block::Card(pod) => card::height(ctx, pod, tab),
        Block::Gap => ctx.tokens.md,
        _ => row,
    }
}

/// Each section the object has, in order, its heading then what it holds unless folded.
fn blocks<'a>(
    app: &AppState,
    (tab, object): (&Opened, &'a Described),
    events: &'a [Arc<Described>],
    now: groove_types::Timestamp,
) -> Vec<Block<'a>> {
    let parts = parts(app, (tab, object), events, now);
    let mut out = Vec::new();
    for (section, count, held) in parts {
        out.push(Block::Heading(section, count));
        if !tab.shut.contains(&section) {
            out.extend(held);
        }
        out.push(Block::Gap);
    }
    out
}

/// Each section with what it says of itself and what it holds.
fn parts<'a>(
    app: &AppState,
    (tab, object): (&Opened, &'a Described),
    events: &'a [Arc<Described>],
    now: groove_types::Timestamp,
) -> Vec<(Section, String, Vec<Block<'a>>)> {
    let pod = object.pod.as_deref();
    let rows = events.iter().filter_map(|one| one.event.as_ref());
    let warned = rows.clone().filter(|one| one.warning).count();
    let mut parts: Vec<(Section, String, Vec<Block<'a>>)> = Vec::new();
    if let Some(pod) = pod {
        parts.push((
            Section::Conditions,
            String::new(),
            vec![Block::Conditions(pod)],
        ));
    }
    parts.push((
        Section::Summary,
        String::new(),
        vec![Block::Summary(sections::facts(app, tab, object, now))],
    ));
    parts.push((
        Section::Relations,
        String::new(),
        vec![Block::Relations(relations::of(app, tab, object))],
    ));
    if let Some(pod) = pod {
        let count = pod.containers.len().to_string();
        parts.push((Section::Containers, count, vec![Block::Card(pod)]));
    }
    let pairs = |all: &'a [(String, String)]| all.iter().map(Block::Pair).collect::<Vec<_>>();
    parts.push((
        Section::Labels,
        object.labels.len().to_string(),
        pairs(&object.labels),
    ));
    parts.push((
        Section::Annotations,
        object.annotations.len().to_string(),
        pairs(&object.annotations),
    ));
    let said = format!("{} · {warned} warning", events.len());
    parts.push((Section::Events, said, rows.map(Block::Event).collect()));
    parts
}

/// The key column of labels and annotations: as wide as the widest key, within reason.
fn key_width(ctx: &mut Ctx, object: &Described) -> f32 {
    let style = ctx.styles.small(Role::Ghost);
    let keys = object
        .labels
        .iter()
        .chain(&object.annotations)
        .map(|(key, _)| key);
    let widest = keys.map(|key| ctx.measure(key, &style)).fold(0.0, f32::max);
    widest.min(ctx.tokens.aside_far) + ctx.tokens.lg
}

/// The events about the object, warnings first, then the latest.
fn events(app: &AppState, tab: &Opened, object: &Described) -> Vec<Arc<Described>> {
    let key = events_key(app, &tab.link, &object.uid);
    let held = key.and_then(|key| app.cluster.store.follows.get(&key));
    let mut events: Vec<Arc<Described>> = held.map(|one| one.objects.clone()).unwrap_or_default();
    let rank = |one: &Arc<Described>| {
        let row = one.event.as_ref();
        let warning = row.is_some_and(|row| row.warning);
        (!warning, std::cmp::Reverse(row.and_then(|row| row.last)))
    };
    events.sort_by_key(rank);
    events
}

/// The pod's usage as last read, when the cluster runs a metrics-server.
fn usage(app: &AppState, tab: &Opened) -> Option<Usage> {
    let link = &tab.link;
    let pod = (
        link.context.clone(),
        link.namespace.clone().unwrap_or_default(),
        link.name.clone(),
    );
    app.cluster.store.follows.usage(&pod)?.0.clone()
}
