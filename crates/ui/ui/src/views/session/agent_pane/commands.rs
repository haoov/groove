//! A glance's commands and MCP calls on a card: the last ones by their first line, or, opened,
//! every one whole with what it gave back.

use groove_gfx::{Edges, Rect};
use groove_types::chat::Run;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::band;
use groove_ui_kit::text::row;
use groove_ui_kit::widgets::{glass_hoverable, glass_tinted};

use crate::ctx::Ctx;
use crate::hit::Target;

/// The last commands a closed card shows.
const COMMANDS: usize = 5;

/// One line of the card: what leads it when it starts a call, its text, and its colour.
type Said = (Option<String>, String, Role);

/// What leads a call's first line: a prompt for a command, the tool's own name for an MCP call.
fn lead(run: &Run) -> String {
    match &run.tool {
        Some(tool) => tool.rsplit("__").next().unwrap_or(tool).to_string(),
        None => "$".to_string(),
    }
}

/// The card's lines.
fn said(ran: &[Run], opened: bool) -> Vec<Said> {
    if !opened {
        let shown = &ran[ran.len().saturating_sub(COMMANDS)..];
        let last = shown.len().saturating_sub(1);
        return shown
            .iter()
            .enumerate()
            .map(|(at, one)| {
                let first = one.command.lines().next().unwrap_or_default().to_string();
                let role = if at == last { Role::Muted } else { Role::Ghost };
                (Some(lead(one)), first, role)
            })
            .collect();
    }
    let mut out = Vec::new();
    for one in ran {
        let mut lines: Vec<String> = one.command.lines().map(str::to_string).collect();
        if lines.is_empty() {
            lines.push(String::new());
        }
        let leads = std::iter::once(Some(lead(one))).chain(std::iter::repeat(None));
        out.extend(
            leads
                .zip(lines)
                .map(|(lead, line)| (lead, line, Role::Text)),
        );
        let printed = if one.failed { Role::Bad } else { Role::Faint };
        let output = one.output.as_deref().unwrap_or_default().lines();
        out.extend(output.map(|line| (None, line.to_string(), printed)));
    }
    out
}

pub(super) fn height(ctx: &Ctx, ran: &[Run], opened: bool) -> f32 {
    said(ran, opened).len() as f32 * ctx.tokens.code_line + ctx.tokens.sm * 2.0
}

/// The card in `room`; a click on it opens or closes it.
pub(super) fn draw(ctx: &mut Ctx, room: Rect, ran: &[Run], opened: bool) {
    let Some(first) = ran.first() else {
        return;
    };
    glass_tinted(ctx, room, ctx.styles.glass_code_ground());
    glass_hoverable(ctx, room, Target::Commands(first.id.clone()));
    let inside = room.pad(Edges::xy(ctx.tokens.md, ctx.tokens.sm));
    let line = ctx.tokens.code_line;
    let said = said(ran, opened);
    let (shell, called) = (
        ctx.styles.code(Role::Attention),
        ctx.styles.code(Role::Accent),
    );
    let gap = ctx.measure(" ", &shell);
    let mut indent = 0.0;
    ctx.clipped(inside, |ctx| {
        for (at, (lead, text, role)) in said.into_iter().enumerate() {
            let span = band(inside, at as f32 * line, line);
            if let Some(lead) = lead {
                let style = if lead == "$" { shell } else { called };
                row(ctx, span, 0.0, &lead, style);
                indent = ctx.measure(&lead, &style) + gap;
            }
            row(ctx, span, indent, &text, ctx.styles.code(role));
        }
    });
}
