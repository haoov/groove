//! A glance's commands and MCP calls on a card: the last few cut to a line, or opened, each whole and wrapped with its output.

use groove_gfx::{Edges, Rect};
use groove_types::chat::Run;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::band;
use groove_ui_kit::text::{elide, row};
use groove_ui_kit::widgets::{glass_hoverable, glass_tinted};

use crate::ctx::Ctx;
use crate::hit::Target;
use crate::views::session::diff::wrap::pieces;

/// The last commands a closed card shows.
const COMMANDS: usize = 5;

/// One line of the card: what leads it when it starts a call, its text, and its colour.
type Said = (Option<String>, String, Role);

/// What leads a call's first line: a prompt for a command, the tool's own name for an MCP call.
fn lead(run: &Run) -> String {
    run.tool_name().unwrap_or("$").to_string()
}

/// The card's lines; opened, each wrapped to what `cols` characters leave past its lead.
fn said(ran: &[Run], opened: bool, cols: usize) -> Vec<Said> {
    if !opened {
        let shown = &ran[ran.len().saturating_sub(COMMANDS)..];
        let last = shown.len().saturating_sub(1);
        return shown
            .iter()
            .enumerate()
            .map(|(at, one)| {
                let first = one.command.lines().next().unwrap_or_default().to_string();
                let role = match (one.failed, at == last) {
                    (true, _) => Role::Bad,
                    (false, true) => Role::Muted,
                    (false, false) => Role::Ghost,
                };
                (Some(lead(one)), first, role)
            })
            .collect();
    }
    let mut out = Vec::new();
    for one in ran {
        let lead = lead(one);
        let room = cols.saturating_sub(lead.chars().count() + 1);
        let leads = std::iter::once(Some(lead)).chain(std::iter::repeat(None));
        let command = folded(&one.command, room).into_iter();
        out.extend(
            leads
                .zip(command)
                .map(|(lead, line)| (lead, line, Role::Text)),
        );
        let printed = if one.failed { Role::Bad } else { Role::Faint };
        let output = one.output.as_deref().filter(|out| !out.is_empty());
        let output = output.map(|out| folded(out, room)).unwrap_or_default();
        out.extend(output.into_iter().map(|line| (None, line, printed)));
    }
    out
}

/// The text as rows of at most `cols` characters, as the prompt wraps.
fn folded(text: &str, cols: usize) -> Vec<String> {
    let lines: Vec<Vec<char>> = text.split('\n').map(|one| one.chars().collect()).collect();
    let rows = pieces(text, cols).into_iter();
    rows.map(|(line, from, to)| lines[line][from..to].iter().collect())
        .collect()
}

/// How many characters a line of a card `width` wide holds.
fn cols(ctx: &Ctx, width: f32) -> usize {
    ((width - ctx.tokens.md * 2.0) / ctx.advance)
        .floor()
        .max(1.0) as usize
}

pub(super) fn height(ctx: &Ctx, ran: &[Run], opened: bool, width: f32) -> f32 {
    let count = said(ran, opened, cols(ctx, width)).len();
    count as f32 * ctx.tokens.code_line + ctx.tokens.sm * 2.0
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
    let said = said(ran, opened, cols(ctx, room.w));
    let (shell, called) = (
        ctx.styles.code(Role::Attention),
        ctx.styles.code(Role::Accent),
    );
    let mut indent = 0.0;
    ctx.clipped(inside, |ctx| {
        for (at, (lead, text, role)) in said.into_iter().enumerate() {
            let span = band(inside, at as f32 * line, line);
            if let Some(lead) = lead {
                let style = if lead == "$" { shell } else { called };
                row(ctx, span, 0.0, &lead, style);
                indent = (lead.chars().count() + 1) as f32 * ctx.advance;
            }
            let style = ctx.styles.code(role);
            let text = match opened {
                true => text,
                false => elide(ctx, &text, &style, span.w - indent),
            };
            row(ctx, span, indent, &text, style);
        }
    });
}
