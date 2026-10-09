//! A scope picker's panel, under the picker: a search line, then each context's lines.

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::base::tokens::PALETTE_ROWS;
use groove_ui_kit::layout::{Spec, column_in};
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{Button, Corner, Row, Text, input, list, panel_at};

use crate::ctx::Ctx;
use crate::hit::{Picks, Target};
use crate::views::session::resources::{Pick, ScopeLine, Scoping, scope_lines};

pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &crate::Ui, scoping: &Scoping) {
    let Some(open) = app.session.selected() else {
        return;
    };
    let under = |which| ctx.app.hits.rect_of(&Target::Picker(which));
    let Some(anchor) = under(scoping.which).or_else(|| under(Picks::Contexts)) else {
        return;
    };
    let lines = scope_lines(app, open, &ui.session.resources, scoping);
    let labels: Vec<String> = lines.iter().map(ScopeLine::label).collect();
    let shown = lines.len().clamp(1, PALETTE_ROWS);
    let height = ctx.tokens.row * (shown + 1) as f32 + ctx.tokens.sm;
    let width = wide(ctx, &labels);
    let at = (anchor.x, anchor.bottom());
    let rect = panel_at(
        ctx,
        at,
        Corner::TopLeft,
        (width, height),
        ctx.styles.border(),
    );
    ctx.hit(rect, Target::ScopePanel);
    let tall = |height: f32| Spec::default().height(height);
    let [_, line, rest] = column_in(
        rect,
        [tall(ctx.tokens.xs), tall(ctx.tokens.row), Spec::fill()],
    );
    let prefix = match scoping.which {
        Picks::Contexts => "context: ",
        _ => "namespace: ",
    };
    input(ctx, line, prefix, scoping.query.text());
    let hairline = ctx.tokens.hairline;
    let body = Rect {
        h: ctx.tokens.row * shown as f32,
        ..rest
    }
    .pad(Edges::across(hairline, hairline));
    if lines.is_empty() {
        let hint = match scoping.which {
            Picks::Contexts => "no context in Settings",
            _ => "no match",
        };
        let room = body.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
        Label::new(hint, ctx.styles.small(Role::Faint)).draw(ctx, room);
        return;
    }
    rows(ctx, app, (body, ui, scoping), (&lines, &labels));
}

/// The widest label, with a check's room before it and a ×'s after.
fn wide(ctx: &mut Ctx, labels: &[String]) -> f32 {
    let style = ctx.styles.label(Role::Text);
    let widest = labels
        .iter()
        .map(|one| ctx.measure(one, &style))
        .fold(0.0, f32::max);
    let marks = (ctx.tokens.icon + ctx.tokens.sm) * 2.0;
    (widest + marks + ctx.tokens.md * 2.0).max(ctx.tokens.menu)
}

/// One line: a context's name in its hue, a held line behind its check, the rest set in.
fn item<'a>(
    ctx: &Ctx,
    app: &AppState,
    held: &crate::views::session::resources::ResourcesUi,
    (at, line, label): (usize, &ScopeLine, &'a str),
) -> Row<'a, Target> {
    let pad = ctx.tokens.md;
    match line {
        ScopeLine::Heading(context) => {
            let hue = app.config.cluster(context).map(|one| Role::Hue(one.hue));
            Row::new(pad, label, ctx.styles.small(hue.unwrap_or(Role::Faint)))
        }
        ScopeLine::Held(pick) => {
            let (mark, role) = match pick.shown(held) {
                true => (Mark::Ticked, Role::Ok),
                false => (Mark::Unticked, Role::Faint),
            };
            let style = ctx.styles.label(Role::Text);
            Row::new(pad, label, style)
                .mark(mark)
                .tint(ctx.styles.label(role).color)
                .mark_size(ctx.tokens.small)
                .target(Target::ScopeLine(at))
        }
        _ => {
            let past = pad + ctx.tokens.icon + ctx.tokens.sm;
            Row::new(past, label, ctx.styles.label(Role::Muted)).target(Target::ScopeLine(at))
        }
    }
}

/// The lines around the cursor, as many as the panel shows; a held one ends in its ×.
fn rows(
    ctx: &mut Ctx,
    app: &AppState,
    (body, ui, scoping): (Rect, &crate::Ui, &Scoping),
    (lines, labels): (&[ScopeLine], &[String]),
) {
    let first = scoping
        .cursor
        .unwrap_or_default()
        .saturating_sub(PALETTE_ROWS - 1);
    let held = &ui.session.resources;
    let items: Vec<Row<'_, Target>> = lines
        .iter()
        .zip(labels)
        .enumerate()
        .skip(first)
        .take(PALETTE_ROWS)
        .map(|(at, (line, label))| item(ctx, app, held, (at, line, label)))
        .collect();
    let selected = scoping.cursor.map(|at| at - first);
    list(ctx, body, &items, selected);
    for (row, (at, line)) in lines
        .iter()
        .enumerate()
        .skip(first)
        .take(PALETTE_ROWS)
        .enumerate()
    {
        if matches!(line, ScopeLine::Held(Pick::Context(_) | Pick::Pair(_))) {
            let line = Rect {
                y: body.y + ctx.tokens.row * row as f32,
                h: ctx.tokens.row,
                ..body
            };
            let mut room = line.pad(Edges::across(0.0, ctx.tokens.sm));
            let close = Button::icon(Mark::Close, 0, Target::ScopeDetach(at), Role::Bad);
            close
                .text(Text::Small)
                .hover(ctx.styles.hover())
                .right(ctx, &mut room, 0.0);
        }
    }
}
