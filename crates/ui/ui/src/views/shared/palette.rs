//! The panel every choice is made in: the palette in the middle, a picker on what
//! it belongs to.

use groove_controllers::AppState;
use groove_gfx::Rect;

use crate::ctx::Ctx;
use crate::hit::Target;
use crate::palette::Palette;
use crate::style::Role;
use crate::tokens::PALETTE_ROWS;
use crate::widget::{Row, input, list, modal, panel_at, row};

/// Whether the panel offers a line to type in: the palette always does, a picker
/// once its list is longer than it can show or something has been typed.
fn asks(palette: &Palette, prompt: &Option<crate::palette::Prompt>, rows: usize) -> bool {
    let free = prompt.as_ref().is_some_and(|prompt| prompt.free);
    palette.anchor.is_none() || free || rows > PALETTE_ROWS || !palette.query.is_empty()
}

/// How wide an anchored panel stands: its widest row, padded, at least a menu's width.
fn wide(ctx: &mut Ctx, rows: &[(String, String)]) -> f32 {
    let style = ctx.styles.label(Role::Text);
    let widest = rows
        .iter()
        .map(|(group, label)| ctx.measure(&format!("{group}{label}"), &style))
        .fold(0.0, f32::max);
    (widest + ctx.tokens.md * 2.0).max(ctx.tokens.menu)
}

/// The command palette: an input line, then the entries or the prompt's options.
pub fn draw(ctx: &mut Ctx, app: &AppState, palette: &Palette) {
    let prompt = palette.prompt(app);
    let rows = listed(app, palette, &prompt);
    let shown = rows.len().clamp(1, PALETTE_ROWS);
    let asks = asks(palette, &prompt, rows.len());
    let height = ctx.tokens.row * (shown + usize::from(asks)) as f32 + ctx.tokens.sm;
    let rect = match palette.anchor {
        Some(anchor) => {
            let width = wide(ctx, &rows);
            panel_at(ctx, anchor.point(), anchor.corner, (width, height))
        }
        None => modal(ctx, ctx.tokens.modal, height, ctx.tokens.modal_top),
    };
    ctx.hit(rect, Target::Palette);
    let under = match asks {
        true => asked(ctx, rect, palette, &prompt),
        false => rect,
    };
    let body = inset(ctx, under, shown);
    match rows.is_empty() {
        true => nothing(ctx, body, &prompt),
        false => items(ctx, body, palette, &prompt, &rows),
    }
}

/// What the panel lists: a prompt's options, or every entry the query leaves.
fn listed(
    app: &AppState,
    palette: &Palette,
    prompt: &Option<crate::palette::Prompt>,
) -> Vec<(String, String)> {
    match prompt {
        Some(_) => palette.options(app),
        None => palette
            .rows(app)
            .into_iter()
            .map(|entry| (entry.group.to_string(), entry.label))
            .collect(),
    }
}

/// The line to type in, at the panel's top. Returns what is left under it.
fn asked(
    ctx: &mut Ctx,
    rect: Rect,
    palette: &Palette,
    prompt: &Option<crate::palette::Prompt>,
) -> Rect {
    let line = Rect::new(rect.x, rect.y + ctx.tokens.xs, rect.w, ctx.tokens.row);
    let prefix = match prompt {
        Some(prompt) => format!("{}: ", prompt.label),
        None => "> ".to_string(),
    };
    input(ctx, line, &prefix, &palette.query);
    Rect::new(rect.x, line.bottom(), rect.w, rect.bottom() - line.bottom())
}

/// The rows' own room, inside the panel's edges.
fn inset(ctx: &Ctx, body: Rect, shown: usize) -> Rect {
    let hairline = ctx.tokens.hairline;
    Rect::new(
        body.x + hairline,
        body.y,
        body.w - hairline * 2.0,
        ctx.tokens.row * shown as f32,
    )
}

/// What stands in the list's place when it is empty.
fn nothing(ctx: &mut Ctx, body: Rect, prompt: &Option<crate::palette::Prompt>) {
    let hint = match prompt {
        Some(prompt) if prompt.free => "Enter to confirm",
        Some(_) => "nothing to pick",
        None => "no match",
    };
    let style = ctx.styles.small(Role::Faint);
    row(ctx, body, ctx.tokens.md, hint, style);
}

/// The rows around the selected one, as far as the panel shows.
fn items(
    ctx: &mut Ctx,
    body: Rect,
    palette: &Palette,
    prompt: &Option<crate::palette::Prompt>,
    rows: &[(String, String)],
) {
    let pad = ctx.tokens.md;
    let first = palette.selected.saturating_sub(PALETTE_ROWS - 1);
    let (group_style, label_style) = (ctx.styles.small(Role::Faint), ctx.styles.label(Role::Text));
    let at = ctx.tokens.aside_near;
    let items: Vec<Row<'_>> = rows
        .iter()
        .skip(first)
        .take(PALETTE_ROWS)
        .enumerate()
        .map(|(i, (group, label))| {
            let row = match prompt {
                Some(_) => Row::new(pad, group, label_style),
                None => Row::new(pad, group, group_style).aside(at, label, label_style),
            };
            row.target(Target::PaletteRow(first + i))
        })
        .collect();
    list(ctx, body, &items, Some(palette.selected - first));
}
