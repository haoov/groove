//! The conversation's column, laid from its end up: what shows at the scroll, and how far it scrolls.

use groove_controllers::session_service::Open;
use groove_gfx::{Edges, Rect};
use groove_types::chat::{Chat, Entry, Glance};
use groove_ui_kit::base::ctx::Run;
use groove_ui_kit::widgets::text_band;

use super::entry::{self, Row};
use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::read::{self, Line};
use crate::hit::{Scroller, Target};
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::{band, from_bottom};
use groove_ui_kit::text::row;

const EMPTY: &str = "The conversation shows here once the agent writes its transcript.";

/// The rows that show, resting between the top bar and `ends`, and going on under the glass at both.
pub(super) fn draw(
    ctx: &mut Ctx,
    (open, chat, working): (&Open, Option<&Chat>, bool),
    ui: &Ui,
    ends: f32,
) {
    let pane = ctx.app.layout.agent;
    let above = Edges {
        top: ctx.app.layout.board.y - pane.y,
        ..Edges::all(0.0)
    };
    let room = pane.pad(above);
    ctx.hit(room, Target::Chat);
    let (entries, queued) = chat.map_or((&[][..], &[][..]), |one| (one.entries(), one.queued()));
    let mut body = pane;
    from_bottom(&mut body, pane.bottom() - ends + ctx.tokens.md, 0.0);
    let body = body.pad(Edges::xy(ctx.tokens.md, ctx.tokens.sm));
    if entries.is_empty() && queued.is_empty() {
        let style = ctx.styles.body(Role::Faint);
        return row(ctx, band(body, 0.0, ctx.tokens.row), 0.0, EMPTY, style);
    }
    let pending = pending(chat, working);
    let mut extent = 0.0;
    let runs = ctx.read(|ctx| {
        ctx.clipped(room, |ctx| {
            let shown = queued.iter().rev().map(|one| Row::Queued(one));
            let shown = shown.chain(pending.iter().map(Row::Entry));
            let shown = shown.chain(entries.iter().rev().map(Row::Entry));
            let live = working.then_some(queued.len());
            let seen = (open, live, &ui.agent.opened[..]);
            extent = rows(ctx, (seen, shown), ui.agent.scroll, (body, room))
        })
    });
    ctx.app.hits.scrolls(Scroller::Chat, extent);
    let lines = runs
        .into_iter()
        .filter(|one| one.rect.y < room.bottom() && one.rect.bottom() > room.y)
        .map(|one| line(ctx, one))
        .collect();
    ctx.app.hits.reading(lines);
    held(ctx, ui, room);
}

/// The rows from the end up, as far as the room reaches; returns how far they scroll.
fn rows<'a>(
    ctx: &mut Ctx,
    ((open, live, opened), shown): (Seen, impl Iterator<Item = Row<'a>>),
    offset: f32,
    (body, room): (Rect, Rect),
) -> f32 {
    let gap = ctx.tokens.md;
    let (mut foot, mut content) = (body.h + offset, 0.0);
    for (at, one) in shown.enumerate() {
        let tall = entry::height(ctx, one, (body.w, opened));
        let top = foot - tall;
        if top < room.bottom() - body.y && foot > room.y - body.y {
            let seen = (open, live == Some(at), opened);
            entry::draw(ctx, seen, one, (band(body, top, tall), room));
        }
        content += tall + gap;
        foot = top - gap;
        if foot < room.y - body.y {
            return offset + body.h;
        }
    }
    (content - gap - body.h).max(0.0)
}

/// The session, the row that is live, by its place from the end, and the glances opened.
type Seen<'a> = (&'a Open, Option<usize>, &'a [String]);

/// While the agent works past its last glance, a glance of its own that runs until a record says more.
fn pending(chat: Option<&Chat>, working: bool) -> Option<Entry> {
    let last = chat.and_then(|one| one.entries().last());
    if !working || matches!(last, Some(Entry::Glance(_))) {
        return None;
    }
    Some(Entry::Glance(Glance {
        thoughts: 1,
        since: chat.and_then(Chat::now),
        ..Glance::default()
    }))
}

/// A run drawn, with where each of its characters starts.
fn line(ctx: &mut Ctx, run: Run) -> Line {
    let mut edges = vec![0.0];
    let mut buf = [0; 4];
    for one in run.text.chars() {
        let wide = ctx.measure(one.encode_utf8(&mut buf), &run.style);
        edges.push(edges.last().copied().unwrap_or_default() + wide);
    }
    Line {
        rect: run.rect,
        text: run.text,
        edges,
    }
}

/// What a drag holds of the conversation, banded under its text.
fn held(ctx: &mut Ctx, ui: &Ui, room: Rect) {
    let Some((from, to)) = ui.agent.reading else {
        return;
    };
    let scroll = ui.agent.scroll;
    let points = ((from.0, from.1 + scroll), (to.0, to.1 + scroll));
    let lines = ctx.app.hits.read().to_vec();
    let color = ctx.styles.held();
    ctx.clipped(room, |ctx| {
        for (at, start, end) in read::held(&lines, points) {
            let line = &lines[at];
            text_band(ctx, line.rect, (line.edges[start], line.edges[end]), color);
        }
    });
}
