//! One block of the transcript: how tall it stands at a width, and how it draws.

use groove_controllers::session_service::Open;
use groove_gfx::{Align, Edges, Rect};
use groove_types::Timestamp;
use groove_types::chat::{Entry, Tool};
use groove_ui_kit::base::style::Role;
use groove_ui_kit::markdown::{self, prose};
use groove_ui_kit::shape::{band, from_top, mid_rule, strip};
use groove_ui_kit::text::{Label, ago, row, rows, wrapped};

use super::parts::{tinted, who};
use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::widgets::{cap, cap_right};

/// A block the transcript draws: an entry with when it was said, a run of edits, or a message
/// waiting for the agent's turn.
#[derive(Clone, Copy)]
pub(super) enum Row<'a> {
    /// `head` when it opens the agent's turn.
    Entry {
        entry: &'a Entry,
        at: Option<Timestamp>,
        head: bool,
    },
    Edits(&'a [Entry]),
    Queued(&'a str),
}

/// How a block is seen: the session, whether it is the live one, and the blocks opened.
pub(super) type Seen<'a> = (&'a Open, bool, &'a [String]);

pub(super) fn height(
    ctx: &mut Ctx,
    shown: Row,
    (width, live, opened): (f32, bool, &[String]),
) -> f32 {
    let (entry, head) = match shown {
        Row::Queued(text) => return you_height(ctx, text, width),
        Row::Edits(edits) => return super::edits::height(ctx, edits),
        Row::Entry { entry, head, .. } => (entry, head),
    };
    match entry {
        Entry::You(text) => you_height(ctx, text, width),
        Entry::Said(text) => {
            let said = markdown::height(ctx, text, width, Role::Text);
            if head { said + who_height(ctx) } else { said }
        }
        Entry::Glance(glance) => super::steps::height(ctx, glance, (live, opened)),
        Entry::Subagent(sub) => super::subagent::height(ctx, sub, opened.contains(&sub.id), width),
        Entry::Failed(text) => failed_height(ctx, text, width),
        Entry::Edit(_) => super::edits::height(ctx, std::slice::from_ref(entry)),
        Entry::Tool(_) | Entry::Interrupted => ctx.tokens.row,
    }
}

/// One block in `whole`, `visible` the part of the column in sight.
pub(super) fn draw(ctx: &mut Ctx, seen: Seen, shown: Row, (whole, visible): (Rect, Rect)) {
    let (open, live, opened) = seen;
    let (entry, at, head) = match shown {
        Row::Queued(text) => return you(ctx, whole, text, None),
        Row::Edits(edits) => return super::edits::draw(ctx, open, whole, edits),
        Row::Entry { entry, at, head } => (entry, at, head),
    };
    match entry {
        Entry::You(text) => you(ctx, whole, text, Some(at)),
        Entry::Said(text) => said(ctx, (whole, visible), text, head.then_some(at)),
        Entry::Glance(one) => super::steps::draw(ctx, whole, one, (live, opened)),
        Entry::Edit(_) => super::edits::draw(ctx, open, whole, std::slice::from_ref(entry)),
        Entry::Tool(tool) => called(ctx, whole, tool),
        Entry::Interrupted => interrupted(ctx, whole, at),
        Entry::Subagent(sub) => super::subagent::draw(ctx, whole, sub, opened.contains(&sub.id)),
        Entry::Failed(text) => failed(ctx, whole, text, at),
    }
}

fn who_height(ctx: &Ctx) -> f32 {
    ctx.tokens.line + ctx.tokens.xs
}

/// The agent's words, plain, under `AGENT · 2m` when they open its turn.
fn said(
    ctx: &mut Ctx,
    (whole, visible): (Rect, Rect),
    text: &str,
    head: Option<Option<Timestamp>>,
) {
    let mut room = whole;
    if let Some(at) = head {
        let line = from_top(&mut room, ctx.tokens.line, ctx.tokens.xs);
        who(ctx, line, ("agent", Role::Accent), at);
    }
    let link = |url: &str| Target::Link(url.to_string());
    prose(ctx, (room, visible), text, Role::Text, link);
}

fn you_height(ctx: &mut Ctx, text: &str, width: f32) -> f32 {
    let style = ctx.styles.body(Role::Text);
    let inner = width - ctx.tokens.lg * 2.0;
    let lines = rows(ctx, text, &style, inner).max(1);
    lines as f32 * ctx.tokens.line + who_height(ctx) + ctx.tokens.md * 2.0
}

/// The user's words across the column on a quiet ground, under `YOU · 2m`; a message still
/// waiting (`at` none) stands faint and says so.
fn you(ctx: &mut Ctx, whole: Rect, text: &str, at: Option<Option<Timestamp>>) {
    let quiet = groove_ui_kit::shape::Panel::default().ground(ctx.styles.hover());
    quiet.border(ctx.styles.panel_edge()).draw(ctx, whole);
    let mut room = whole.pad(Edges::xy(ctx.tokens.lg, ctx.tokens.md));
    let line = from_top(&mut room, ctx.tokens.line, ctx.tokens.xs);
    let role = match at {
        Some(at) => {
            who(ctx, line, ("you", Role::Muted), at);
            Role::Text
        }
        None => {
            let mut line = line;
            cap(ctx, &mut line, "you", Role::Ghost);
            let waits = Label::new("waits for the agent", ctx.styles.log(Role::Ghost));
            waits.draw(ctx, line);
            Role::Faint
        }
    };
    let style = ctx.styles.body(role);
    let height = ctx.tokens.line;
    for (n, one) in wrapped(ctx, text, &style, room.w).iter().enumerate() {
        row(ctx, band(room, n as f32 * height, height), 0.0, one, style);
    }
}

/// A rule across the column, `Interrupted by you · 2m` in its middle.
fn interrupted(ctx: &mut Ctx, whole: Rect, at: Option<Timestamp>) {
    let said = match at {
        Some(at) => format!("Interrupted by you · {}", ago(at.age_at(ctx.now))),
        None => "Interrupted by you".to_string(),
    };
    let style = ctx.styles.log(Role::Ghost);
    let wide = ctx.measure(&said, &style) + ctx.tokens.md * 2.0;
    let middle = whole.align((wide, whole.h), Align::Center, Align::Center);
    let edge = ctx.styles.panel_edge();
    mid_rule(ctx, strip(whole, 0.0, (whole.w - wide) / 2.0), edge);
    mid_rule(
        ctx,
        strip(whole, (whole.w + wide) / 2.0, (whole.w - wide) / 2.0),
        edge,
    );
    let text = middle.align(
        (wide - ctx.tokens.md * 2.0, whole.h),
        Align::Center,
        Align::Center,
    );
    Label::new(&said, style).draw(ctx, text);
}

/// What went wrong: its first line as the title, the rest under it.
fn told(text: &str) -> (&str, &str) {
    let (title, rest) = text.split_once('\n').unwrap_or((text, ""));
    (title.trim_end_matches([':', '.']), rest.trim())
}

fn failed_height(ctx: &mut Ctx, text: &str, width: f32) -> f32 {
    let (_, rest) = told(text);
    let style = ctx.styles.code(Role::Faint);
    let lines = match rest.is_empty() {
        true => 0,
        false => rows(ctx, rest, &style, width - ctx.tokens.md * 2.0),
    };
    ctx.tokens.row + lines as f32 * ctx.tokens.code_line + ctx.tokens.sm * 2.0
}

/// A red-tinted block: what failed in small capitals and when, then what it printed.
fn failed(ctx: &mut Ctx, whole: Rect, text: &str, at: Option<Timestamp>) {
    tinted(ctx, whole, Role::Bad);
    let mut room = whole.pad(Edges::xy(ctx.tokens.md, ctx.tokens.sm));
    let mut line = from_top(&mut room, ctx.tokens.row, 0.0);
    let (title, rest) = told(text);
    if let Some(at) = at {
        let said = ago(at.age_at(ctx.now));
        Label::new(&said, ctx.styles.log(Role::Ghost)).right(ctx, &mut line, ctx.tokens.sm);
    }
    cap(ctx, &mut line, title, Role::Bad);
    let style = ctx.styles.code(Role::Faint);
    let height = ctx.tokens.code_line;
    for (n, one) in wrapped(ctx, rest, &style, room.w).iter().enumerate() {
        row(ctx, band(room, n as f32 * height, height), 0.0, one, style);
    }
}

/// Another tool: its name in small capitals, what it was given, and how it stands.
fn called(ctx: &mut Ctx, whole: Rect, tool: &Tool) {
    let mut room = whole;
    let (word, role) = match tool.failed {
        None => ("running", Role::Working),
        Some(true) => ("failed", Role::Bad),
        Some(false) => ("done", Role::Ghost),
    };
    cap_right(ctx, &mut room, word, role);
    cap(ctx, &mut room, &tool.name, Role::Faint);
    if let Some(detail) = &tool.detail {
        Label::new(detail, ctx.styles.code(Role::Muted)).draw(ctx, room);
    }
}
