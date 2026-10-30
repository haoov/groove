//! What a headless agent asks, on glass over the prompt: leave to run a tool, or the options of its question.

use groove_gfx::{Edges, Rect};
use groove_types::chat::{Permission, Question};
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::band;
use groove_ui_kit::text::{elide, row};
use groove_ui_kit::widgets::{Button, glass_pane, offer};

use crate::ctx::Ctx;
use crate::hit::{Answering, Target};

/// Where the conversation ends: over the card while one asks, else over the prompt.
pub(super) fn ends(ctx: &Ctx, asked: Option<(&Permission, usize)>, prompt: Rect) -> f32 {
    match asked {
        Some((one, picked)) => card(ctx, prompt, lines(one, picked)).y,
        None => prompt.y,
    }
}

/// The first ask, its question `picked` stands on, over the prompt.
pub(super) fn draw(ctx: &mut Ctx, asked: &Permission, picked: usize, prompt: Rect) {
    let card = card(ctx, prompt, lines(asked, picked));
    glass_pane(ctx, card);
    let inside = card.pad(Edges::xy(ctx.tokens.md, ctx.tokens.sm));
    let tall = ctx.tokens.row;
    match asked.questions.get(picked) {
        Some(question) => asking(ctx, question, inside),
        None => leave(
            ctx,
            asked,
            (band(inside, 0.0, tall), band(inside, tall, tall)),
        ),
    }
}

/// A question's own row and one for each option; two for a tool's leave.
fn lines(asked: &Permission, picked: usize) -> usize {
    let question = asked.questions.get(picked);
    question.map_or(2, |one| one.options.len() + 1)
}

/// `lines` rows of the card, standing the prompt's width just over it.
fn card(ctx: &Ctx, prompt: Rect, lines: usize) -> Rect {
    let (row, sm) = (ctx.tokens.row, ctx.tokens.sm);
    let tall = row * lines as f32 + sm * 2.0;
    band(prompt, -(sm + tall), tall)
}

/// The tool by name, what it acts on under it, and the two answers.
fn leave(ctx: &mut Ctx, asked: &Permission, (top, below): (Rect, Rect)) {
    let mut room = top;
    offer(ctx, &mut room, "refuse", Target::Answer(Answering::Refuse));
    offer(ctx, &mut room, "allow", Target::Answer(Answering::Allow));
    let title = format!("Allow {}?", named(&asked.tool));
    let style = ctx.styles.body(Role::Warn);
    let title = elide(ctx, &title, &style, room.w);
    row(ctx, room, 0.0, &title, style);
    let style = ctx.styles.code(Role::Muted);
    let detail = asked.detail.as_deref().unwrap_or_default();
    let detail = elide(ctx, detail, &style, below.w);
    row(ctx, below, 0.0, &detail, style);
}

/// The question, then its options one a row, numbered as their keys pick them.
fn asking(ctx: &mut Ctx, question: &Question, inside: Rect) {
    let tall = ctx.tokens.row;
    let (style, top) = (ctx.styles.body(Role::Text), band(inside, 0.0, tall));
    let said = elide(ctx, &question.question, &style, top.w);
    row(ctx, top, 0.0, &said, style);
    for (at, option) in question.options.iter().enumerate() {
        let mut room = band(inside, tall * (at + 1) as f32, tall);
        let label = format!("{} {option}", at + 1);
        let target = Target::Answer(Answering::Pick(option.clone()));
        let button = Button::glass(&label, target, Role::Muted, &ctx.styles).flat();
        button.left(ctx, &mut room, 0.0);
    }
}

/// An MCP tool by its server and its own name.
fn named(tool: &str) -> String {
    match tool.strip_prefix("mcp__") {
        Some(rest) => rest.replacen("__", " · ", 1).replace('_', " "),
        None => tool.to_string(),
    }
}
