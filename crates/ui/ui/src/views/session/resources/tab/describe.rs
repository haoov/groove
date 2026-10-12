//! The described view: one column of sections, each under a heading that folds it.

use std::sync::Arc;

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};
use groove_types::{
    AppCondition, AppSource, Container, Described, Operation, PodPart, SyncedResource, Usage,
};
use groove_ui_kit::base::style::Role;
use groove_ui_kit::widgets::scrolled;

use super::super::around::events_key;
use super::super::opened::{Opened, Section};
use super::argo;
use super::containers;
use super::destined;
use super::relations::{self, Line};
use super::sections::{self, Fact};
use crate::ctx::Ctx;
use crate::hit::Scroller;

/// One block of the column, stacked from the top.
pub(super) enum Block<'a> {
    Heading(Section, Tally),
    Operation(&'a Operation),
    Said(&'a str),
    Broke(&'a SyncedResource),
    AppCondition(&'a AppCondition),
    Source(&'a AppSource),
    Conditions(&'a PodPart),
    Summary(Vec<Fact>),
    /// A line of a table, under its header when it is the first.
    Relation(Line, bool),
    Container(&'a Container, bool),
    /// A line of an unfolded container.
    Detail((&'static str, String)),
    Pair(&'a (String, String)),
    Event(&'a groove_types::EventRow),
    Gap,
    /// The hairline over a section that follows another.
    Rule,
}

/// What a section's heading says after its name.
pub(super) enum Tally {
    Count(usize),
    Note(String),
}

pub(super) fn draw(
    ctx: &mut Ctx,
    body: Rect,
    app: &AppState,
    (tab, object): (&Opened, &Described),
) {
    let events = events(app, tab, object);
    let blocks = blocks(app, (tab, object), &events, ctx.now);
    let md = ctx.tokens.md;
    let usage = usage(app, tab);
    let keys = key_width(ctx, object, body.w - md * 2.0);
    let heights: Vec<f32> = blocks.iter().map(|one| height(ctx, one)).collect();
    let indexed: Vec<(usize, &Block)> = blocks.iter().enumerate().collect();
    let tint = tinted(object, &blocks);
    let mut tip = None;
    let extent = scrolled(
        ctx,
        body,
        tab.scroll,
        &indexed,
        |(at, _)| heights[*at],
        |ctx, line, (at, block)| {
            if let Some((role, _)) = tint.filter(|(_, until)| at < until) {
                argo::ground(ctx, line, role);
            }
            let rect = line.pad(Edges::across(md, md));
            let drawn = one(ctx, rect, block, (tab, keys, usage.as_ref()));
            tip = drawn.or(tip.take());
        },
    );
    ctx.app.hits.scrolls(Scroller::Resources, extent);
    if let Some(tip) = &tip {
        containers::tip(ctx, tip);
    }
}

/// One block in `rect`; a container's row returns the tip under the pointer.
fn one(
    ctx: &mut Ctx,
    rect: Rect,
    block: &Block,
    (tab, keys, usage): (&Opened, f32, Option<&Usage>),
) -> Option<containers::Tip> {
    let last = tab.copied.as_deref();
    match block {
        Block::Heading(section, tally) => {
            sections::heading(ctx, rect, (*section, tab.shut.contains(section)), tally)
        }
        Block::Operation(_) | Block::Said(_) | Block::Broke(_) => operated(ctx, rect, block),
        Block::AppCondition(one) => argo::condition(ctx, rect, one, ctx.now),
        Block::Source(one) => argo::source(ctx, rect, one, last),
        Block::Conditions(pod) => sections::conditions(ctx, rect, pod),
        Block::Summary(facts) => sections::grid(ctx, rect, facts, last),
        Block::Relation(line, header) => relations::draw(ctx, rect, (line, *header), last),
        Block::Container(one, header) => {
            return containers::row(ctx, rect, (tab, *one, *header), usage);
        }
        Block::Detail(line) => containers::detail(ctx, rect, line, last),
        Block::Pair(pair) => sections::pair(ctx, rect, pair, (keys, last)),
        Block::Event(row) => sections::event(ctx, rect, row, last),
        Block::Gap => {}
        Block::Rule => groove_ui_kit::shape::top_rule(ctx, rect, ctx.styles.line()),
    }
    None
}

/// The operation's colour, and the blocks it stands under: from the top to the rule after it.
fn tinted(object: &Described, blocks: &[Block]) -> Option<(Role, usize)> {
    let role = object.app.as_deref().and_then(argo::tint)?;
    let until = blocks.iter().position(|one| matches!(one, Block::Rule));
    Some((role, until.unwrap_or(blocks.len())))
}

/// A row of the operation.
fn operated(ctx: &mut Ctx, rect: Rect, block: &Block) {
    match block {
        Block::Operation(one) => argo::operation(ctx, rect, one, ctx.now),
        Block::Said(message) => argo::said(ctx, rect, message),
        Block::Broke(one) => argo::broke(ctx, rect, one),
        _ => {}
    }
}

fn height(ctx: &Ctx, block: &Block) -> f32 {
    let row = ctx.tokens.row;
    match block {
        Block::Summary(facts) => row * facts.len() as f32,
        Block::Relation(_, true) | Block::Container(_, true) => row * 2.0,
        Block::Source(one) => row * argo::source_rows(one) as f32,
        Block::Gap => ctx.tokens.md,
        Block::Rule => ctx.tokens.sm,
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
    let mut out = vec![Block::Gap];
    for (section, count, held) in parts {
        if out.len() > 1 {
            out.extend([Block::Gap, Block::Rule]);
        }
        out.push(Block::Heading(section, count));
        if !tab.shut.contains(&section) {
            out.extend(held);
        }
    }
    out.push(Block::Gap);
    out
}

/// Each section with what it says of itself and what it holds.
fn parts<'a>(
    state: &AppState,
    (tab, object): (&Opened, &'a Described),
    events: &'a [Arc<Described>],
    now: groove_types::Timestamp,
) -> Vec<(Section, Tally, Vec<Block<'a>>)> {
    let pod = object.pod.as_deref();
    let app = object.app.as_deref();
    let mut parts: Vec<(Section, Tally, Vec<Block<'a>>)> = Vec::new();
    parts.extend(app.map(argo::parts).unwrap_or_default());
    if let Some(pod) = pod {
        parts.push((
            Section::Conditions,
            Tally::Count(0),
            vec![Block::Conditions(pod)],
        ));
    }
    let facts = match app {
        Some(one) => {
            let context = destined::context(state, &tab.link.context, &one.destination);
            argo::facts(one, context, state.cluster.found.is_some())
        }
        None => sections::facts(state, tab, object, now),
    };
    parts.push((
        Section::Summary,
        Tally::Count(0),
        vec![Block::Summary(facts)],
    ));
    parts.extend(app.map(argo::sourced));
    if let Some(pod) = pod {
        let count = Tally::Count(pod.containers.len());
        parts.push((Section::Containers, count, contained(tab, pod)));
    }
    let [labels, annotations, events] = around(object, events);
    parts.extend([labels, annotations]);
    if app.is_none() {
        let lines = relations::lines(state, tab, object);
        let count = Tally::Count(relations::total(&lines));
        let blocks = lines.into_iter().enumerate();
        let blocks = blocks.map(|(at, line)| Block::Relation(line, at == 0));
        parts.push((Section::Relations, count, blocks.collect()));
    }
    parts.push(events);
    parts
}

/// A row each container, the unfolded ones followed by their lines.
fn contained<'a>(tab: &Opened, pod: &'a PodPart) -> Vec<Block<'a>> {
    let mut out = Vec::new();
    for (at, one) in pod.containers.iter().enumerate() {
        out.push(Block::Container(one, at == 0));
        if !containers::folded(tab, one) {
            out.extend(containers::details(one).into_iter().map(Block::Detail));
        }
    }
    out
}

/// What every object has: its labels, its annotations, the events about it.
fn around<'a>(
    object: &'a Described,
    events: &'a [Arc<Described>],
) -> [(Section, Tally, Vec<Block<'a>>); 3] {
    let rows = events.iter().filter_map(|one| one.event.as_ref());
    let warned = rows.clone().filter(|one| one.warning).count();
    let pairs = |all: &'a [(String, String)]| all.iter().map(Block::Pair).collect::<Vec<_>>();
    let said = Tally::Note(format!("{} · {warned} warning", events.len()));
    [
        (
            Section::Labels,
            Tally::Count(object.labels.len()),
            pairs(&object.labels),
        ),
        (
            Section::Annotations,
            Tally::Count(object.annotations.len()),
            pairs(&object.annotations),
        ),
        (Section::Events, said, rows.map(Block::Event).collect()),
    ]
}

/// The key column of labels and annotations: as wide as the widest key, half of `room` at most.
fn key_width(ctx: &mut Ctx, object: &Described, room: f32) -> f32 {
    let style = ctx.styles.small(Role::Ghost);
    let keys = object
        .labels
        .iter()
        .chain(&object.annotations)
        .map(|(key, _)| key);
    let widest = keys.map(|key| ctx.measure(key, &style)).fold(0.0, f32::max);
    (widest + ctx.tokens.lg).min(room / 2.0)
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
