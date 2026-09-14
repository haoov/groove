use groove_controllers::AppState;
use groove_gfx::Rect;

use crate::ctx::Ctx;
use crate::palette::Palette;
use crate::style::Role;
use crate::tokens::PALETTE_ROWS;
use crate::widget::{Row, input, list, modal, row};

/// The command palette: an input line, then the entries or the prompt's options.
pub fn draw(ctx: &mut Ctx, app: &AppState, palette: &Palette) {
    let prompt = palette.prompt(app);
    let rows: Vec<(String, String)> = match &prompt {
        Some(_) => palette.options(app),
        None => palette
            .rows(app)
            .into_iter()
            .map(|entry| (entry.group.to_string(), entry.label))
            .collect(),
    };
    let shown = rows.len().clamp(1, PALETTE_ROWS);
    let (row_height, pad) = (ctx.tokens.row, ctx.tokens.md);
    let height = row_height * (shown + 1) as f32 + ctx.tokens.sm;
    let rect = modal(ctx, ctx.tokens.modal, height, ctx.tokens.modal_top);

    let line = Rect::new(rect.x, rect.y + ctx.tokens.xs, rect.w, row_height);
    let prefix = match &prompt {
        Some(prompt) => format!("{}: ", prompt.label),
        None => "> ".to_string(),
    };
    input(ctx, line, &prefix, &palette.query);

    let hairline = ctx.tokens.hairline;
    let body = Rect::new(
        rect.x + hairline,
        line.bottom(),
        rect.w - hairline * 2.0,
        row_height * shown as f32,
    );
    if rows.is_empty() {
        let hint = match &prompt {
            Some(prompt) if prompt.free => "Enter to confirm",
            Some(_) => "nothing to pick",
            None => "no match",
        };
        let style = ctx.styles.small(Role::Faint);
        return row(ctx, body, pad, hint, style);
    }

    let first = palette.selected.saturating_sub(PALETTE_ROWS - 1);
    let (group_style, label_style) = (ctx.styles.small(Role::Faint), ctx.styles.label(Role::Text));
    let at = ctx.tokens.aside_near;
    let items: Vec<Row<'_>> = rows
        .iter()
        .skip(first)
        .take(PALETTE_ROWS)
        .map(|(group, label)| match &prompt {
            Some(_) => Row::new(pad, group, label_style),
            None => Row::new(pad, group, group_style).aside(at, label, label_style),
        })
        .collect();
    list(ctx, body, &items, Some(palette.selected - first));
}
