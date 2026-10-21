//! The panel every choice is made in: the palette, or a picker on what it belongs to.

mod rows;
mod scoped;

use rows::{entry, matched};

use groove_controllers::AppState;
use std::ops::Range;

use groove_gfx::{Edges, Rect};

use crate::ctx::Ctx;
use crate::hit::Target;
use crate::keymap::Keymap;
use crate::palette::Palette;
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::base::tokens::{PALETTE_LINES, PALETTE_ROWS};
use groove_ui_kit::layout::{Spec, column_in};
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{Lead, Material, Row, glass_list, modal, panel_at, stepped_input};

/// Lines the panel shows: more centred than on a picker.
fn limit(palette: &Palette) -> usize {
    match palette.anchor {
        Some(_) => PALETTE_ROWS,
        None => PALETTE_LINES,
    }
}

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
            false => pad + ctx.tokens.icon + ctx.tokens.sm + ctx.measure(label, &style),
        })
        .fold(0.0, f32::max);
    (widest + pad).max(ctx.tokens.menu) + ctx.tokens.sm * 2.0
}

/// The command palette: an input line, then the entries or the prompt's options.
pub fn draw(ctx: &mut Ctx, app: &AppState, palette: &Palette) {
    let prompt = palette.prompt(app);
    let rows = listed(app, palette, &prompt);
    let lines = lines(&rows, prompt.is_none());
    let typing_only = rows.is_empty() && prompt.as_ref().is_some_and(|one| one.free);
    let shown = lines.len().clamp(usize::from(!typing_only), limit(palette));
    let typed = has_input(palette, &prompt, rows.len());
    let scope = scoped::scope(app, palette);
    let pinned = ctx.tokens.row * f32::from(u8::from(scope.is_some()));
    let rows_high = ctx.tokens.glass_row * shown as f32 + pinned + ctx.tokens.sm * 2.0;
    let height = match typed {
        true => rows_high + ctx.tokens.bar + ctx.tokens.xs * 2.0,
        false => rows_high,
    };
    let edge = (ctx.styles.border(), Material::Glass);
    let rect = match palette.anchor {
        Some(anchor) => {
            let width = width(ctx, &rows, prompt.is_some());
            panel_at(ctx, anchor.point(), anchor.corner, (width, height), edge)
        }
        None => {
            let size = (ctx.tokens.palette, height);
            modal(ctx, size, ctx.tokens.modal_top, edge)
        }
    };
    ctx.hit(rect, Target::Palette);
    let under = match typed {
        true => input_line(ctx, rect, palette, &prompt),
        false => rect,
    };
    let under = match &scope {
        Some(words) => scoped::draw(ctx, under, words),
        None => under,
    };
    let body = inset(ctx, under, shown);
    let ends = (
        shortcuts(app, palette, &prompt),
        marks(app, palette, &prompt),
    );
    match rows.is_empty() {
        true if typing_only => {}
        true => nothing(ctx, body, palette, &prompt),
        false => items(ctx, body, (palette, &prompt), (&rows, &lines), ends),
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

fn marks(
    app: &AppState,
    palette: &Palette,
    prompt: &Option<crate::palette::Prompt>,
) -> Vec<Option<Mark>> {
    if prompt.is_some() {
        return Vec::new();
    }
    let rows = palette.rows(app).into_iter();
    rows.map(|entry| crate::palette::mark_of(&entry, app))
        .collect()
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
    let [_, line, _, rest] = column_in(
        rect,
        [
            tall(ctx.tokens.xs),
            tall(ctx.tokens.bar),
            tall(ctx.tokens.xs),
            Spec::fill(),
        ],
    );
    let query = &palette.query;
    match prompt {
        Some(prompt) => {
            let hint = match (prompt.allow_empty && query.is_empty(), prompt.free) {
                (true, _) => Some("Enter for the default"),
                (false, true) => Some("Enter to confirm"),
                (false, false) => None,
            };
            let typed = (query.text(), query.before(), hint);
            stepped_input(ctx, line, Lead::Step(prompt.label), typed);
        }
        None => {
            let hint = match query.text() {
                "" => Some("# Files"),
                text if text.starts_with('#') => Some("Files"),
                _ => None,
            };
            let typed = (query.text(), query.before(), hint);
            stepped_input(ctx, line, Lead::Mark(Mark::Search), typed);
        }
    }
    rest
}

/// The rows' own room, inset from the panel's edges.
fn inset(ctx: &Ctx, body: Rect, shown: usize) -> Rect {
    let pad = ctx.tokens.sm;
    let gap = Spec::default().height(pad);
    let tall = Spec::default().height(ctx.tokens.glass_row * shown as f32);
    let [_, rows, _] = column_in(body, [gap, tall, Spec::fill()]);
    rows.pad(Edges::across(pad, pad))
}

/// What stands in the list's place when it is empty.
fn nothing(ctx: &mut Ctx, body: Rect, palette: &Palette, prompt: &Option<crate::palette::Prompt>) {
    let hint = match prompt {
        Some(_) => "Nothing to pick".to_string(),
        None => format!("No match for \u{201c}{}\u{201d}", palette.query.text()),
    };
    let room = body.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
    Label::new(&hint, ctx.styles.body(Role::Faint)).draw(ctx, room);
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Line {
    Head(usize),
    Row(usize),
}

fn lines(rows: &[(String, String)], headed: bool) -> Vec<Line> {
    let mut out = Vec::with_capacity(rows.len());
    for (i, (group, _)) in rows.iter().enumerate() {
        let starts = i == 0 || rows[i - 1].0 != *group;
        if headed && starts {
            out.push(Line::Head(i));
        }
        out.push(Line::Row(i));
    }
    out
}

/// The selected row in view, with its heading when it fits.
fn first_line(lines: &[Line], selected: usize, limit: usize) -> usize {
    let at = lines
        .iter()
        .position(|line| *line == Line::Row(selected))
        .unwrap_or(0);
    let head = lines[..at]
        .iter()
        .rposition(|line| matches!(line, Line::Head(_)))
        .unwrap_or(at);
    let base = at.saturating_sub(limit - 1);
    match base > head && at - head < limit {
        true => head,
        false => base,
    }
}

fn items(
    ctx: &mut Ctx,
    body: Rect,
    (palette, prompt): (&Palette, &Option<crate::palette::Prompt>),
    (rows, lines): (&[(String, String)], &[Line]),
    (chords, marks): (Vec<Option<String>>, Vec<Option<Mark>>),
) {
    let first = first_line(lines, palette.selected, limit(palette));
    let head_style = ctx.styles.heading(Role::Faint);
    let heads: Vec<String> = rows.iter().map(|(group, _)| group.to_uppercase()).collect();
    let texts: Vec<&str> = rows
        .iter()
        .map(|(group, label)| if prompt.is_some() { group } else { label }.as_str())
        .collect();
    let query = palette.query.text();
    let fuzzy = prompt.is_some();
    let found: Vec<Vec<Range<usize>>> = texts
        .iter()
        .map(|text| matched(text, query, fuzzy))
        .collect();
    let shown = &lines[first..lines.len().min(first + limit(palette))];
    let lit = palette.anchor.is_none() || palette.keyed;
    let pad = ctx.tokens.md;
    let items: Vec<Row<'_, _>> = shown
        .iter()
        .map(|line| match *line {
            Line::Head(i) => Row::new(pad, &heads[i], head_style),
            Line::Row(i) => {
                let chord = chords.get(i).and_then(Option::as_deref);
                let mark = marks.get(i).copied().flatten();
                let selected = lit && palette.selected == i;
                entry(ctx, texts[i], (chord, mark, &found[i]), selected)
                    .target(Target::PaletteRow(i))
            }
        })
        .collect();
    let at = shown
        .iter()
        .position(|line| *line == Line::Row(palette.selected));
    glass_list(ctx, body, &items, at.filter(|_| lit));
}
