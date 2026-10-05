//! The panel every choice is made in: the palette, or a picker on what it belongs to.

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};

use crate::ctx::Ctx;
use crate::hit::Target;
use crate::keymap::Keymap;
use crate::palette::Palette;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::base::tokens::PALETTE_ROWS;
use groove_ui_kit::layout::{Spec, column_in};
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{Row, input, list, modal, panel_at};

/// Whether the panel offers a line to type in: the palette always, a picker once needed.
fn has_input(palette: &Palette, prompt: &Option<crate::palette::Prompt>, rows: usize) -> bool {
    let free = prompt.as_ref().is_some_and(|prompt| prompt.free);
    palette.anchor.is_none() || free || rows > PALETTE_ROWS || !palette.query.is_empty()
}

/// How wide an anchored panel stands: its widest drawn row, at least a menu's width.
fn width(ctx: &mut Ctx, rows: &[(String, String)], prompted: bool) -> f32 {
    let style = ctx.styles.label(Role::Text);
    let pad = ctx.tokens.md;
    let widest = rows
        .iter()
        .map(|(group, label)| match prompted {
            true => pad + ctx.measure(group, &style),
            false => ctx.tokens.aside_near + ctx.measure(label, &style),
        })
        .fold(0.0, f32::max);
    (widest + pad).max(ctx.tokens.menu)
}

/// The command palette: an input line, then the entries or the prompt's options.
pub fn draw(ctx: &mut Ctx, app: &AppState, palette: &Palette) {
    let prompt = palette.prompt(app);
    let rows = listed(app, palette, &prompt);
    let shown = rows.len().clamp(1, PALETTE_ROWS);
    let typed = has_input(palette, &prompt, rows.len());
    let rows_high = ctx.tokens.row * (shown + usize::from(typed)) as f32;
    let height = match typed {
        true => rows_high + ctx.tokens.sm,
        false => rows_high,
    };
    let edge = ctx.styles.border();
    let rect = match palette.anchor {
        Some(anchor) => {
            let width = width(ctx, &rows, prompt.is_some());
            panel_at(ctx, anchor.point(), anchor.corner, (width, height), edge)
        }
        None => modal(ctx, ctx.tokens.modal, height, ctx.tokens.modal_top, edge),
    };
    ctx.hit(rect, Target::Palette);
    let under = match typed {
        true => input_line(ctx, rect, palette, &prompt),
        false => rect,
    };
    let body = inset(ctx, under, shown);
    let chords = shortcuts(app, palette, &prompt);
    match rows.is_empty() {
        true => nothing(ctx, body, &prompt),
        false => items(ctx, body, (palette, &prompt), &rows, &chords),
    }
}

/// Each entry's chord, as the keymap binds it; a prompt's options have none.
fn shortcuts(
    app: &AppState,
    palette: &Palette,
    prompt: &Option<crate::palette::Prompt>,
) -> Vec<Option<String>> {
    if prompt.is_some() {
        return Vec::new();
    }
    let keymap = Keymap::of(app.config.config.as_ref());
    let rows = palette.rows(app).into_iter();
    rows.map(|entry| keymap.label_of(entry.id())).collect()
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
fn input_line(
    ctx: &mut Ctx,
    rect: Rect,
    palette: &Palette,
    prompt: &Option<crate::palette::Prompt>,
) -> Rect {
    let tall = |height: f32| Spec::default().height(height);
    let [_, line, rest] = column_in(
        rect,
        [tall(ctx.tokens.xs), tall(ctx.tokens.row), Spec::fill()],
    );
    let prefix = match prompt {
        Some(prompt) => format!("{}: ", prompt.label),
        None => "> ".to_string(),
    };
    input(ctx, line, &prefix, &palette.query);
    rest
}

/// The rows' own room, inside the panel's edges.
fn inset(ctx: &Ctx, body: Rect, shown: usize) -> Rect {
    let hairline = ctx.tokens.hairline;
    let rows = Rect {
        h: ctx.tokens.row * shown as f32,
        ..body
    };
    rows.pad(Edges::across(hairline, hairline))
}

/// What stands in the list's place when it is empty.
fn nothing(ctx: &mut Ctx, body: Rect, prompt: &Option<crate::palette::Prompt>) {
    let hint = match prompt {
        Some(prompt) if prompt.free => "Enter to confirm",
        Some(_) => "nothing to pick",
        None => "no match",
    };
    let room = body.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
    Label::new(hint, ctx.styles.small(Role::Faint)).draw(ctx, room);
}

/// The rows around the selected one, as far as the panel shows.
fn items(
    ctx: &mut Ctx,
    body: Rect,
    (palette, prompt): (&Palette, &Option<crate::palette::Prompt>),
    rows: &[(String, String)],
    chords: &[Option<String>],
) {
    let pad = ctx.tokens.md;
    let first = palette.selected.saturating_sub(PALETTE_ROWS - 1);
    let (group_style, label_style) = (ctx.styles.small(Role::Faint), ctx.styles.label(Role::Text));
    let at = ctx.tokens.aside_near;
    let items: Vec<Row<'_, _>> = rows
        .iter()
        .enumerate()
        .skip(first)
        .take(PALETTE_ROWS)
        .map(|(i, (group, label))| {
            let row = match prompt {
                Some(_) => Row::new(pad, group, label_style),
                None => Row::new(pad, group, group_style).aside(at, label, label_style),
            };
            let row = match chords.get(i).and_then(Option::as_deref) {
                Some(chord) => row.end(chord, group_style),
                None => row,
            };
            row.target(Target::PaletteRow(i))
        })
        .collect();
    let shown = palette.anchor.is_none() || palette.keyed;
    let selected = shown.then(|| palette.selected - first);
    list(ctx, body, &items, selected);
}
