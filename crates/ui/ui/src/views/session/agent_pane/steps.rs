//! Reads, searches, commands and calls back to back, folded into one block: the step running now
//! over it, counted in words with the last step under; opened, each step with what it printed.

use groove_gfx::{Edges, Rect};
use groove_types::chat::{Glance, Step};
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::{Panel, band, from_left, from_top, top_rule};
use groove_ui_kit::text::{Label, lasted, row, wrapped};
use groove_ui_kit::widgets::{fold, glass_hoverable};

use super::parts::{boxed, tally, tinted};
use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::widgets::cap;

/// The last lines of a command's output an opened block shows.
const OUTPUT_LINES: usize = 8;

/// What opens or folds the block: its first command, else when it began.
pub(crate) fn key(glance: &Glance) -> String {
    match glance.runs().next() {
        Some(run) => run.id.clone(),
        None => format!("glance@{}", glance.since.map_or(0, |at| at.seconds())),
    }
}

fn output(step: &Step) -> Option<&str> {
    let run = step.run.as_ref()?;
    run.output.as_deref().filter(|one| !one.trim().is_empty())
}

/// The last lines of what `text` printed.
fn tail(text: &str) -> Vec<&str> {
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(OUTPUT_LINES)..].to_vec()
}

/// The sunk box's room under a step's row.
fn sunk(ctx: &Ctx, text: &str) -> f32 {
    tail(text).len() as f32 * ctx.tokens.code_line + ctx.tokens.sm * 2.0 + ctx.tokens.xs
}

pub(super) fn height(ctx: &Ctx, glance: &Glance, (_, opened): (bool, &[String])) -> f32 {
    let mut tall = 0.0;
    if !glance.steps.is_empty() {
        tall += ctx.tokens.row + ctx.tokens.line + ctx.tokens.xs;
    }
    if opened.contains(&key(glance)) {
        let rows = glance.steps.len() as f32 * ctx.tokens.row;
        let printed: f32 = glance
            .steps
            .iter()
            .filter_map(output)
            .map(|one| sunk(ctx, one))
            .sum();
        tall += rows + printed;
    }
    tall
}

/// The block; what runs now stands in the agent's bar at the middle's foot.
pub(super) fn draw(ctx: &mut Ctx, whole: Rect, glance: &Glance, (_, opened): (bool, &[String])) {
    if !glance.steps.is_empty() {
        folded(ctx, whole, glance, opened.contains(&key(glance)));
    }
}

/// The box: the chevron, the tally and the time, the last step under; opened, every step.
fn folded(ctx: &mut Ctx, block: Rect, glance: &Glance, open: bool) {
    boxed(ctx, block);
    glass_hoverable(ctx, block, Target::Commands(key(glance)));
    let mut room = block.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
    let mut top = from_top(&mut room, ctx.tokens.row, 0.0);
    fold(ctx, &mut top, open, Role::Ghost);
    if let Some(took) = took(glance) {
        Label::new(&took, ctx.styles.log(Role::Ghost)).right(ctx, &mut top, 0.0);
    }
    tally(ctx, &mut top, glance);
    let last = from_top(&mut room, ctx.tokens.line, ctx.tokens.xs);
    if let Some(step) = glance.steps.last() {
        Label::new(&step.target, ctx.styles.log(Role::Faint)).draw(ctx, last);
    }
    if open {
        table(ctx, room, glance);
    }
}

/// A row for each step, its kind, target and time; a command's output in a sunk box under it.
fn table(ctx: &mut Ctx, mut room: Rect, glance: &Glance) {
    for (at, step) in glance.steps.iter().enumerate() {
        let mut line = from_top(&mut room, ctx.tokens.row, 0.0);
        top_rule(ctx, line, ctx.styles.panel_edge());
        let mut kind = from_left(&mut line, ctx.tokens.aside_near, ctx.tokens.sm);
        cap(ctx, &mut kind, step.kind.word(), Role::Faint);
        if let Some(took) = glance.took(at).filter(|one| !one.is_zero()) {
            let took = lasted(took);
            Label::new(&took, ctx.styles.log(Role::Ghost)).right(ctx, &mut line, 0.0);
        }
        let failed = step.run.as_ref().is_some_and(|one| one.failed);
        let role = if failed { Role::Bad } else { Role::Muted };
        Label::new(&step.target, ctx.styles.code(role)).draw(ctx, line);
        if let Some(text) = output(step) {
            let tall = sunk(ctx, text);
            let printed = from_top(&mut room, tall, 0.0);
            let indent = ctx.tokens.aside_near + ctx.tokens.sm;
            let printed = printed.pad(Edges {
                left: indent,
                bottom: ctx.tokens.xs,
                ..Edges::all(0.0)
            });
            sunk_box(ctx, printed, text, failed);
        }
    }
}

fn sunk_box(ctx: &mut Ctx, rect: Rect, text: &str, failed: bool) {
    let panel = Panel::default().ground(ctx.styles.deep());
    panel.border(ctx.styles.panel_edge()).draw(ctx, rect);
    let inside = rect.pad(Edges::xy(ctx.tokens.sm, ctx.tokens.sm));
    let role = if failed { Role::Bad } else { Role::Faint };
    let style = ctx.styles.code(role);
    let line = ctx.tokens.code_line;
    ctx.clipped(inside, |ctx| {
        for (n, one) in tail(text).into_iter().enumerate() {
            let one = wrapped(ctx, one, &style, f32::INFINITY).concat();
            row(ctx, band(inside, n as f32 * line, line), 0.0, &one, style);
        }
    });
}

/// `RUNNING` in the accent, what the agent does now, and how long it has run.
pub(crate) fn running(ctx: &mut Ctx, line: Rect, glance: &Glance) {
    tinted(ctx, line, Role::Accent);
    let mut room = line.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
    let since = glance.steps.last().and_then(|one| one.at).or(glance.since);
    if let Some(since) = since {
        let took = lasted(since.age_at(ctx.now));
        Label::new(&took, ctx.styles.log(Role::Accent)).right(ctx, &mut room, 0.0);
    }
    let word = if glance.running { "running" } else { "working" };
    cap(ctx, &mut room, word, Role::Accent);
    let doing = glance.doing.as_deref().unwrap_or_default();
    Label::new(doing, ctx.styles.code(Role::Muted)).draw(ctx, room);
}

/// How long the block took, to the record after its last step.
fn took(glance: &Glance) -> Option<String> {
    let took = glance.since?.age_at(glance.until?);
    (!took.is_zero()).then(|| lasted(took))
}
