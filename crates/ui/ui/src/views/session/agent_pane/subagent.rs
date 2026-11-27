//! An agent the agent sent: its type, task, state and time, its steps, and its answer.

use groove_gfx::{Align, Edges, Rect};
use groove_types::chat::{Chat, Entry, Sent, Subagent};
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::{Panel, from_right, from_top};
use groove_ui_kit::text::{Label, lasted, row, wrapped};
use groove_ui_kit::widgets::glass_hoverable;

use super::parts::{boxed, head, tally};
use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::widgets::cap;

/// The lines of its answer a closed block shows.
const ANSWER_LINES: usize = 2;

pub(super) fn height(ctx: &mut Ctx, sub: &Subagent, opened: bool, width: f32) -> f32 {
    let inner = width - ctx.tokens.md * 2.0;
    let (lines, cut) = answer(ctx, sub, opened, inner);
    let mut tall = ctx.tokens.row + ctx.tokens.sm;
    if !sub.steps.steps.is_empty() {
        tall += ctx.tokens.row;
    }
    tall += lines.len() as f32 * ctx.tokens.line;
    if cut || opened {
        tall += ctx.tokens.row;
    }
    tall
}

/// The block; a click on it shows its answer whole, or cuts it again.
pub(super) fn draw(ctx: &mut Ctx, whole: Rect, sub: &Subagent, opened: bool) {
    boxed(ctx, whole);
    glass_hoverable(ctx, whole, Target::Commands(sub.id.clone()));
    let mut room = whole;
    let top = head(ctx, from_top(&mut room, ctx.tokens.row, 0.0));
    header(ctx, top, sub);
    let mut room = room.pad(Edges {
        bottom: ctx.tokens.sm,
        ..Edges::across(ctx.tokens.md, ctx.tokens.md)
    });
    if !sub.steps.steps.is_empty() {
        let mut steps = from_top(&mut room, ctx.tokens.row, 0.0);
        tally(ctx, &mut steps, &sub.steps);
    }
    let (lines, cut) = answer(ctx, sub, opened, room.w);
    let style = ctx.styles.body(Role::Muted);
    let line = ctx.tokens.line;
    for one in &lines {
        row(ctx, from_top(&mut room, line, 0.0), 0.0, one, style);
    }
    let link = match (cut, opened) {
        (true, _) => "Show the whole answer",
        (false, true) => "Show less",
        (false, false) => return,
    };
    let at = from_top(&mut room, ctx.tokens.row, 0.0);
    Label::new(link, ctx.styles.small(Role::Accent)).draw(ctx, at);
}

/// Its type and task at the left; its state and how long it took at the right.
pub(super) fn header(ctx: &mut Ctx, mut top: Rect, sub: &Subagent) {
    if let Some(took) = took(ctx, sub) {
        Label::new(&took, ctx.styles.log(Role::Ghost)).right(ctx, &mut top, ctx.tokens.sm);
    }
    pill(ctx, &mut top, sub.state);
    cap(ctx, &mut top, &sub.kind, Role::Muted);
    Label::new(&sub.task, ctx.styles.small(Role::Faint)).draw(ctx, top);
}

/// Its state as a word in small capitals, ringed in its colour, at the right of `room`.
fn pill(ctx: &mut Ctx, room: &mut Rect, sent: Sent) {
    let (word, role) = state(sent);
    let word = word.to_uppercase();
    let style = ctx.styles.tag(role);
    let wide = ctx.measure(&word, &style) + ctx.tokens.sm * 2.0;
    let slot = from_right(room, wide, ctx.tokens.sm);
    let ring = slot.align((wide, ctx.tokens.line), Align::Center, Align::Center);
    Panel::default()
        .border(ctx.styles.band_edge(role))
        .draw(ctx, ring);
    let text = ring.pad(Edges::across(ctx.tokens.sm, ctx.tokens.sm));
    Label::new(&word, style).draw(ctx, text);
}

/// How it stands, in a word and its colour.
pub(super) fn state(sent: Sent) -> (&'static str, Role) {
    match sent {
        Sent::Working => ("working", Role::Accent),
        Sent::Done => ("done", Role::Ok),
        Sent::Failed => ("failed", Role::Bad),
        Sent::Stopped => ("stopped", Role::Warn),
    }
}

/// How long it worked: to its answer, or to now while it works.
pub(super) fn took(ctx: &Ctx, sub: &Subagent) -> Option<String> {
    let since = sub.since?;
    let until = match sub.state {
        Sent::Working => ctx.now,
        _ => sub.until?,
    };
    let took = since.age_at(until);
    (!took.is_zero()).then(|| lasted(took))
}

/// Its answer wrapped to `width`, whole when opened, else its first lines; and whether it was cut.
fn answer(ctx: &mut Ctx, sub: &Subagent, opened: bool, width: f32) -> (Vec<String>, bool) {
    let Some(text) = sub.answer.as_deref().filter(|one| !one.is_empty()) else {
        return (Vec::new(), false);
    };
    let style = ctx.styles.body(Role::Muted);
    let mut lines = wrapped(ctx, text, &style, width);
    let cut = !opened && lines.len() > ANSWER_LINES;
    if cut {
        lines.truncate(ANSWER_LINES);
        if let Some(last) = lines.last_mut() {
            last.push('…');
        }
    }
    (lines, cut)
}

/// The agents the dock follows: every one still working, and those done since the user last spoke.
pub(super) fn docked(chat: Option<&Chat>) -> Vec<&Subagent> {
    let entries = chat.map_or(&[][..], Chat::entries);
    let spoke = entries.iter().rposition(|one| matches!(one, Entry::You(_)));
    let since = spoke.map_or(0, |at| at + 1);
    let sent = entries
        .iter()
        .enumerate()
        .filter_map(|(at, one)| match one {
            Entry::Subagent(sub) => Some((at, sub)),
            _ => None,
        });
    let shown = sent.filter(|(at, sub)| sub.state == Sent::Working || *at >= since);
    shown.map(|(_, sub)| sub).collect()
}
