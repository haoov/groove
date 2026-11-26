//! What the agent asks, over the prompt: leave to run a tool, or its questions one at a time.

use groove_gfx::{Edges, Rect};
use groove_types::chat::{Permission, Question};
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::{Panel, band, from_bottom, from_left, from_top};
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{Button, glass_hoverable, glass_pane};

use super::parts::{cap, keycap};
use crate::ctx::Ctx;
use crate::hit::{Answering, Target};

/// Where the conversation ends: over the card while one asks, else over the prompt.
pub(super) fn ends(ctx: &Ctx, asked: Option<(&Permission, usize)>, prompt: Rect) -> f32 {
    match asked {
        Some((one, picked)) => card(ctx, prompt, one, picked).y,
        None => prompt.y,
    }
}

/// The rows a card takes past its head and its answers' row.
fn body_rows(asked: &Permission, picked: usize) -> f32 {
    match asked.questions.get(picked) {
        Some(question) => (question.options.len() * 2 + 1) as f32,
        None => 1.0,
    }
}

/// The card standing just over the prompt, as wide as it.
fn card(ctx: &Ctx, prompt: Rect, asked: &Permission, picked: usize) -> Rect {
    let (row, sm, md) = (ctx.tokens.row, ctx.tokens.sm, ctx.tokens.md);
    let tall = row * (body_rows(asked, picked) + 2.0) + md * 2.0 + sm;
    band(prompt, -(sm + tall), tall)
}

/// The first ask, its question `picked` stands on, over the prompt.
pub(super) fn draw(ctx: &mut Ctx, asked: &Permission, picked: usize, (prompt, place): (Rect, &str)) {
    let card = card(ctx, prompt, asked, picked);
    glass_pane(ctx, card);
    let ring = Panel::default().border(ctx.styles.band_edge(Role::Attention));
    ring.draw(ctx, card);
    let mut room = card;
    let top = from_top(&mut room, ctx.tokens.row, 0.0);
    Panel::default().ground(ctx.styles.band_tint(Role::Attention)).draw(ctx, top);
    let mut top = top.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
    let room = room.pad(Edges::all(ctx.tokens.md));
    match asked.questions.get(picked) {
        Some(question) => {
            let of = format!("question {} of {}", picked + 1, asked.questions.len());
            cap(ctx, &mut top, &of, Role::Attention);
            Label::new(&question.header, ctx.styles.log(Role::Ghost)).draw(ctx, top);
            asking(ctx, room, question, picked);
        }
        None => {
            let title = format!("{} asks leave", named(&asked.tool));
            cap(ctx, &mut top, &title, Role::Attention);
            Label::new(place, ctx.styles.log(Role::Ghost)).draw(ctx, top);
            leave(ctx, room, asked);
        }
    }
}

/// What the tool runs in a sunk box, then Refuse and Allow on their keys.
fn leave(ctx: &mut Ctx, mut room: Rect, asked: &Permission) {
    let mut acts = from_bottom(&mut room, ctx.tokens.row, ctx.tokens.sm);
    let command = room;
    let sunk = Panel::default().ground(ctx.styles.deep());
    sunk.border(ctx.styles.panel_edge()).draw(ctx, command);
    let inside = command.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
    let detail = asked.detail.as_deref().unwrap_or_default();
    Label::new(detail, ctx.styles.code(Role::Text)).draw(ctx, inside);
    answer(ctx, &mut acts, ("Allow", "ENTER"), (Answering::Allow, true));
    answer(ctx, &mut acts, ("Refuse", "ESC"), (Answering::Refuse, false));
}

/// One answer at the right of `room`: its word and its key's cap; the first in the ask's colour.
fn answer(ctx: &mut Ctx, room: &mut Rect, (word, key): (&str, &str), (said, first): (Answering, bool)) {
    keycap(ctx, room, key);
    let (role, ground) = match first {
        true => (Role::Attention, ctx.styles.band_tint(Role::Attention)),
        false => (Role::Text, ctx.styles.hover()),
    };
    let button = Button::new(word, Target::Answer(said), role, ground);
    let edge = if first { ctx.styles.band_edge(role) } else { ctx.styles.line() };
    button.edged(edge).squared().right(ctx, room, ctx.tokens.md);
}

/// The question, its options each on its digit with what it means, then how to answer.
fn asking(ctx: &mut Ctx, mut room: Rect, question: &Question, picked: usize) {
    let line = from_top(&mut room, ctx.tokens.row, 0.0);
    Label::new(&question.question, ctx.styles.label(Role::Text)).draw(ctx, line);
    let mut acts = from_bottom(&mut room, ctx.tokens.row, ctx.tokens.sm);
    for (at, option) in question.options.iter().enumerate() {
        let two = from_top(&mut room, ctx.tokens.row * 2.0, 0.0);
        let target = Target::Answer(Answering::Pick(option.clone()));
        glass_hoverable(ctx, two, target);
        let mut first = band(two, 0.0, ctx.tokens.row);
        let mut digit = from_left(&mut first, ctx.tokens.lg * 2.0, ctx.tokens.sm);
        keycap(ctx, &mut digit, &(at + 1).to_string());
        Label::new(option, ctx.styles.body(Role::Text)).draw(ctx, first);
        let about = question.described.get(at).map_or("", String::as_str);
        let second = band(two, ctx.tokens.row, ctx.tokens.row);
        let second = second.pad(Edges::across(ctx.tokens.lg * 2.0 + ctx.tokens.sm, 0.0));
        Label::new(about, ctx.styles.small(Role::Faint)).draw(ctx, second);
    }
    if picked > 0 {
        let back = Button::new("Back", Target::Answer(Answering::Back), Role::Text, ctx.styles.hover());
        back.edged(ctx.styles.line()).squared().right(ctx, &mut acts, ctx.tokens.md);
    }
    Label::new("A digit picks.", ctx.styles.small(Role::Faint)).draw(ctx, acts);
}

/// An MCP tool by its server and its own name.
fn named(tool: &str) -> String {
    match tool.strip_prefix("mcp__") {
        Some(rest) => rest.replacen("__", " · ", 1).replace('_', " "),
        None => tool.to_string(),
    }
}
