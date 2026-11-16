//! The described view: one column of sections, each under a heading that folds it.

use std::sync::Arc;

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};
use groove_types::{
    AppCondition, AppSource, Condition, Container, Described, Operation, PodPart, SyncedResource,
    Usage,
};
use groove_ui_kit::base::style::Role;
use groove_ui_kit::widgets::{rounded_slice, scrolled};

use super::super::opened::{Opened, Section};
use super::argo;
use super::containers;
use super::destined;
use super::read::{events, key_width, usage};
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
    Condition(&'a Condition),
    Summary(Vec<Fact>),
    /// A line of a table, under its header when it is the first.
    Relation(Line, bool),
    Container(&'a Container, bool),
    /// A line of an unfolded container.
    Detail((&'static str, String)),
    Pair(&'a (String, String)),
    Event(&'a groove_types::EventRow),
    /// The room between two cards.
    Gap,
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
    let keys = key_width(ctx, object, body.w - (md + md) * 2.0);
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
            if matches!(block, Block::Gap) {
                return;
            }
            let card = line.pad(Edges::across(md, md));
            let tinted = tint.filter(|(_, until)| at < until);
            let gap = |one: Option<&Block>| matches!(one, None | Some(Block::Gap));
            let ends = (gap(blocks.get(at.wrapping_sub(1))), gap(blocks.get(at + 1)));
            carded(ctx, card, (block, ends), tinted.map(|(role, _)| role));
            let rect = match block {
                Block::Heading(..) => card,
                Block::Container(..) | Block::Relation(..) | Block::Detail(_) => {
                    card.pad(Edges::across(ctx.tokens.hairline, ctx.tokens.hairline))
                }
                _ => card.pad(Edges::across(md, md)),
            };
            let drawn = one(ctx, rect, block, (tab, keys, usage.as_ref()));
            tip = drawn.or(tip.take());
        },
    );
    ctx.app.hits.scrolls(Scroller::Resources, extent);
    if let Some(tip) = &tip {
        containers::tip(ctx, tip);
    }
}

/// A slice of its section's card: the sunk ground inside the panel's edge, the heading on its
/// band, ruled off under it unless the section is folded to it alone. The operation's colour
/// tints its card whole, its heading included.
fn carded(ctx: &mut Ctx, card: Rect, (block, ends): (&Block, (bool, bool)), tint: Option<Role>) {
    let ground = match tint {
        Some(role) => ctx.styles.tint(role),
        None => ctx.styles.band(),
    };
    rounded_slice(ctx, card, ends, (ground, Some(ctx.styles.panel_edge())));
    if !matches!(block, Block::Heading(..)) {
        return;
    }
    let alone = ends.1;
    match tint {
        None => groove_ui_kit::widgets::panel_head(ctx, card, alone),
        Some(_) if !alone => groove_ui_kit::shape::hairline(ctx, card, ctx.styles.line()),
        Some(_) => {}
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
        Block::Condition(one) => sections::condition(ctx, rect, one),
        Block::Summary(facts) => sections::grid(ctx, rect, facts, last),
        Block::Relation(line, header) => relations::draw(ctx, rect, (line, *header), last),
        Block::Container(one, header) => {
            return containers::row(ctx, rect, (tab, *one, *header), usage);
        }
        Block::Detail(line) => containers::detail(ctx, rect, line, last),
        Block::Pair(pair) => sections::pair(ctx, rect, pair, (keys, last)),
        Block::Event(row) => sections::event(ctx, rect, row, last),
        Block::Gap => {}
    }
    None
}

/// The operation's colour, and the blocks of the first card it tints.
fn tinted(object: &Described, blocks: &[Block]) -> Option<(Role, usize)> {
    let role = object.app.as_deref().and_then(argo::tint)?;
    let until = blocks
        .iter()
        .skip(1)
        .position(|one| matches!(one, Block::Gap));
    Some((role, until.map_or(blocks.len(), |at| at + 1)))
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
        Block::Heading(..) => row + ctx.tokens.xs,
        Block::Source(one) => row * argo::source_rows(one) as f32,
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
    let mut out = vec![Block::Gap];
    for (section, count, held) in parts {
        if out.len() > 1 {
            out.push(Block::Gap);
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
        let lines = pod.conditions.iter().map(Block::Condition).collect();
        parts.push((Section::Conditions, Tally::Count(0), lines));
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
