//! Groove's own prompt: a pane of glass near the pane's foot, centred, that grows with what is typed.

use groove_gfx::{Align, Edges, Rect};

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::read::Typed;
use crate::hit::{Scroller, Target};
use crate::views::session::Prompt;
use crate::views::session::diff::wrap::{marked, pieces};
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::{Panel, band, from_bottom};
use groove_ui_kit::text::row;
use groove_ui_kit::widgets::{glass_pane, typed_line};

/// The most rows the prompt grows to; past them it scrolls.
const PROMPT_LINES: usize = 8;

const HINT: &str = "Reply to the agent, or / for commands";

/// Where the prompt stands, the line under it, and the rows its text takes.
pub(super) struct Laid {
    pub(super) pane: Rect,
    pub(super) meta: Rect,
    rows: Vec<(usize, usize, usize)>,
    shown: usize,
}

/// The prompt over its line of usage, at the pane's foot.
pub(super) fn laid(ctx: &mut Ctx, ui: &Ui) -> Laid {
    let (sm, xs, md, line) = (
        ctx.tokens.sm,
        ctx.tokens.xs,
        ctx.tokens.md,
        ctx.tokens.code_line,
    );
    let mut room = ctx.app.layout.agent.pad(Edges::across(sm + xs, sm + xs));
    let wide = room.w.min(ctx.tokens.palette);
    room = room.align((wide, room.h), Align::Center, Align::Start);
    let meta = from_bottom(&mut room, ctx.tokens.row, xs);
    let cols = ((room.w - md * 2.0) / ctx.advance).floor().max(1.0) as usize;
    let rows = pieces(&ui.agent.prompt.text(), cols);
    let shown = rows.len().clamp(1, PROMPT_LINES);
    let pane = band(
        room,
        room.h - shown as f32 * line - md * 2.0,
        shown as f32 * line + md * 2.0,
    );
    Laid {
        pane,
        meta,
        rows,
        shown,
    }
}

/// The prompt where `laid` stands it.
pub(super) fn draw(ctx: &mut Ctx, ui: &Ui, laid: &Laid, focused: bool) {
    let (line, prompt) = (ctx.tokens.code_line, &ui.agent.prompt);
    glass_pane(ctx, laid.pane);
    let ring = match focused {
        true => ctx.styles.focus(),
        false => ctx.styles.panel_edge(),
    };
    Panel::default().border(ring).draw(ctx, laid.pane);
    ctx.hit(laid.pane, Target::Prompt);
    let inside = laid.pane.pad(Edges::all(ctx.tokens.md));
    if prompt.is_empty() {
        let hint = ctx.styles.code(Role::Faint);
        row(ctx, band(inside, 0.0, line), 0.0, HINT, hint);
    }
    let scroll = scrolled(prompt, &laid.rows, (laid.shown, line));
    let typed = typed(ctx, prompt, (inside, scroll), (&laid.rows, focused));
    let extent = (laid.rows.len() - laid.shown) as f32 * line;
    ctx.app.hits.scrolls(Scroller::Prompt, extent);
    ctx.app.hits.typing(typed);
}

/// How far the rows scroll: as the wheel left them, or to keep the caret in sight.
fn scrolled(prompt: &Prompt, rows: &[(usize, usize, usize)], (shown, line): (usize, f32)) -> f32 {
    let extent = (rows.len() - shown) as f32 * line;
    let mut scroll = prompt.scroll;
    if prompt.follow {
        let caret = prompt.buffer.caret();
        let at = rows
            .iter()
            .rposition(|&(on, start, _)| on == caret.line && start <= caret.column)
            .unwrap_or_default() as f32;
        let fewest = (at + 1.0) * line - shown as f32 * line;
        scroll = scroll.min(at * line).max(fewest);
    }
    scroll.clamp(0.0, extent)
}

/// The rows in sight, each with what is held on it and the caret where the keys are.
fn typed(
    ctx: &mut Ctx,
    prompt: &Prompt,
    (inside, scroll): (Rect, f32),
    (rows, focused): (&[(usize, usize, usize)], bool),
) -> Typed {
    let (line, style) = (ctx.tokens.code_line, ctx.styles.code(Role::Text));
    let mut drawn = Vec::new();
    ctx.clipped(inside, |ctx| {
        for (at, &piece) in rows.iter().enumerate() {
            let top = at as f32 * line - scroll;
            if top + line <= 0.0 || top >= inside.h {
                continue;
            }
            let span = band(inside, top, line);
            let (body, caret, held) = marked(&prompt.buffer, piece);
            let caret = caret.filter(|_| focused);
            typed_line(ctx, span, (&body, style), (caret, held));
            drawn.push((span, piece));
        }
    });
    Typed {
        rows: drawn,
        scroll,
    }
}
