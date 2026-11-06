//! An agent the agent sent, on a card: its type and task, its own steps, and its answer, cut or whole.

use groove_gfx::{Align, Edges, Rect};
use groove_types::chat::{Chat, Entry, Glance, Sent, Subagent};
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::{band, from_left, from_top};
use groove_ui_kit::text::{Label, lasted, row, wrapped};
use groove_ui_kit::widgets::{busy, glass_hoverable, icon};

use crate::ctx::Ctx;
use crate::hit::Target;

/// The lines of its answer a closed card shows.
const ANSWER_LINES: usize = 2;

pub(super) fn height(ctx: &mut Ctx, sub: &Subagent, opened: bool, width: f32) -> f32 {
    let inner = width - ctx.tokens.md * 2.0;
    let steps = stepped(&sub.steps) as usize;
    let lines = answer(ctx, sub, opened, inner).len();
    let rows = (1 + steps) as f32 * ctx.tokens.row;
    rows + lines as f32 * ctx.tokens.line + ctx.tokens.sm * 2.0
}

/// The card, a click on it opening or closing its answer.
pub(super) fn draw(ctx: &mut Ctx, at: Rect, sub: &Subagent, opened: bool) {
    glass_hoverable(ctx, at, Target::Commands(sub.id.clone()));
    let mut room = at.pad(Edges::xy(ctx.tokens.md, ctx.tokens.sm));
    let head = from_top(&mut room, ctx.tokens.row, 0.0);
    header(ctx, head, sub);
    if stepped(&sub.steps) {
        let steps = from_top(&mut room, ctx.tokens.row, 0.0);
        let live = sub.state == Sent::Working;
        super::glance::summary(
            ctx,
            steps.pad(Edges::across(lead(ctx), 0.0)),
            &sub.steps,
            live,
        );
    }
    let style = ctx.styles.body(Role::Muted);
    let line = ctx.tokens.line;
    for (n, one) in answer(ctx, sub, opened, room.w).iter().enumerate() {
        row(ctx, band(room, n as f32 * line, line), 0.0, one, style);
    }
}

/// Its mark, type and task at the left; how it stands and how long it took at the right.
pub(super) fn header(ctx: &mut Ctx, mut head: Rect, sub: &Subagent) {
    let size = ctx.tokens.icon;
    let slot = from_left(&mut head, size, ctx.tokens.xs);
    let slot = slot.align((size, size), Align::Center, Align::Center);
    let (word, role) = state(sub.state);
    match sub.state {
        Sent::Working => busy(ctx, slot, Role::Accent),
        _ => icon(ctx, slot, Mark::Agent, Role::Accent),
    }
    if let Some(took) = took(ctx, sub) {
        Label::new(&took, ctx.styles.code(Role::Ghost)).right(ctx, &mut head, ctx.tokens.sm);
    }
    Label::new(word, ctx.styles.small(role)).right(ctx, &mut head, ctx.tokens.sm);
    Label::new(&sub.kind, ctx.styles.body(Role::Accent)).left(ctx, &mut head, ctx.tokens.sm);
    Label::new(&sub.task, ctx.styles.body(Role::Text)).draw(ctx, head);
}

/// How it stands, in a word and its colour.
pub(super) fn state(sent: Sent) -> (&'static str, Role) {
    match sent {
        Sent::Working => ("working", Role::Working),
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

/// The room under the header's mark.
fn lead(ctx: &Ctx) -> f32 {
    ctx.tokens.icon + ctx.tokens.xs
}

fn stepped(steps: &Glance) -> bool {
    steps.thoughts > 0 || !steps.looked.is_empty() || !steps.ran.is_empty()
}

/// Its answer wrapped to `width`: whole when opened, else its first lines, the last cut with `…`.
fn answer(ctx: &mut Ctx, sub: &Subagent, opened: bool, width: f32) -> Vec<String> {
    let Some(text) = sub.answer.as_deref().filter(|one| !one.is_empty()) else {
        return Vec::new();
    };
    let style = ctx.styles.body(Role::Muted);
    let mut lines = wrapped(ctx, text, &style, width);
    if !opened && lines.len() > ANSWER_LINES {
        lines.truncate(ANSWER_LINES);
        if let Some(last) = lines.last_mut() {
            last.push('…');
        }
    }
    lines
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
