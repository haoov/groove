//! The agent's prompt, glass over everything near the window's foot, centred as wide as the
//! palette: the commands `/` offers, what is typed, and its keys.

mod commands;

use groove_controllers::AppState;
use groove_gfx::{Align, Edges, Rect};

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::read::Typed;
use crate::hit::{Scroller, Target};
use crate::views::session::Prompt;
use crate::views::session::agent_pane::offered;
use crate::views::session::diff::wrap::{marked, pieces};
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::{band, from_bottom, from_top, top_rule};
use groove_ui_kit::text::{Label, row};
use groove_ui_kit::widgets::keycap;
use groove_ui_kit::widgets::{focus_ring, glass_pane, typed_line};

/// The most rows the prompt grows to; past them it scrolls.
const PROMPT_LINES: usize = 8;

const HINT: &str = "Reply to the agent, or / for commands";

/// The open prompt, dropping into place as it appears by `open`.
pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui, open: f32) {
    let (line, md) = (ctx.tokens.code_line, ctx.tokens.md);
    let wide = ctx.tokens.palette.min(ctx.window.w - ctx.tokens.xl);
    let cols = ((wide - md * 2.0) / ctx.advance).floor().max(1.0) as usize;
    let rows = pieces(&ui.agent.prompt.text(), cols);
    let shown = rows.len().clamp(1, PROMPT_LINES);
    let (offers, own) = offered::grouped(app, ui);
    let listed = commands::height(ctx, (&offers, own), ui.agent.offered);
    let typed_h = shown as f32 * line + md * 2.0;
    let tall = listed + typed_h + ctx.tokens.bar;
    let above = ctx.tokens.row + ctx.tokens.lg;
    let top = ctx.window.h - tall - above + ctx.tokens.sm * (1.0 - open);
    let column = ctx
        .window
        .align((wide, ctx.window.h), Align::Center, Align::Start);
    let panel = band(column, top, tall);
    ctx.layer();
    glass_pane(ctx, panel);
    ctx.hit(panel, Target::Prompt);
    let focused = ui.focus == crate::Focus::Agent;
    if focused {
        focus_ring(ctx, panel);
    }
    let mut room = panel;
    let list = from_top(&mut room, listed, 0.0);
    commands::draw(ctx, list, (&offers, own), ui.agent.offered);
    let keys = from_bottom(&mut room, ctx.tokens.bar, 0.0);
    keyed(ctx, app, keys);
    let inside = room.pad(Edges::all(md));
    let prompt = &ui.agent.prompt;
    if prompt.is_empty() {
        row(
            ctx,
            band(inside, 0.0, line),
            0.0,
            HINT,
            ctx.styles.code(Role::Faint),
        );
    }
    let scroll = scrolled(prompt, &rows, (shown, line));
    let typed = typed(ctx, prompt, (inside, scroll), (&rows, focused));
    ctx.app
        .hits
        .scrolls(Scroller::Prompt, (rows.len() - shown) as f32 * line);
    ctx.app.hits.typing(typed);
}

/// The prompt's keys on their caps, the model and the context at the left.
fn keyed(ctx: &mut Ctx, app: &AppState, keys: Rect) {
    top_rule(ctx, keys, ctx.styles.panel_edge());
    let mut room = keys.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
    let said = [
        ("close", &["ESC"][..]),
        ("new line", &["ENTER", "SHIFT"][..]),
        ("send", &["ENTER"][..]),
    ];
    for (word, caps) in said {
        Label::new(word, ctx.styles.small(Role::Faint)).right(ctx, &mut room, ctx.tokens.xs);
        for cap in caps {
            keycap(ctx, &mut room, cap);
        }
        room = room.pad(Edges::across(0.0, ctx.tokens.md));
    }
    let used = crate::views::session::agent_pane::model::summary(app);
    Label::new(&used, ctx.styles.log(Role::Ghost)).draw(ctx, room);
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
