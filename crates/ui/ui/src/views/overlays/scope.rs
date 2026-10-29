//! The scope panel, where the palette stands: a search line, then each context's lines.

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::base::tokens::PALETTE_LINES;
use groove_ui_kit::layout::{Spec, column_in, column_of};
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{Button, Lead, Material, Row, Text, glass_list, modal, stepped_input};

use crate::ctx::Ctx;
use crate::hit::{Picks, Target};
use crate::views::session::resources::{ScopeLine, Scoping, scope_lines, shows};
use groove_controllers::session_service::Open;

pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &crate::Ui, scoping: &Scoping) {
    let Some(open) = app.session.selected() else {
        return;
    };
    let lines = scope_lines(app, open, scoping);
    let labels: Vec<String> = lines.iter().map(ScopeLine::label).collect();
    let shown = lines.len().clamp(1, PALETTE_LINES);
    let (high, sm, xs) = (ctx.tokens.glass_row, ctx.tokens.sm, ctx.tokens.xs);
    let height = ctx.tokens.bar + xs * 2.0 + high * shown as f32 + sm * 2.0;
    let edge = (ctx.styles.border(), Material::Glass);
    let rect = modal(
        ctx,
        (ctx.tokens.palette, height),
        ctx.tokens.modal_top,
        edge,
    );
    ctx.hit(rect, Target::ScopePanel);
    let tall = |height: f32| Spec::default().height(height);
    let [_, line, _, body, _] = column_in(
        rect,
        [
            tall(xs),
            tall(ctx.tokens.bar),
            tall(sm),
            tall(high * shown as f32),
            Spec::fill(),
        ],
    );
    typed(ctx, line, scoping);
    if lines.is_empty() {
        let hint = match scoping.which {
            Picks::Contexts => "no context in Settings",
            _ => "no match",
        };
        let room = body.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
        Label::new(hint, ctx.styles.small(Role::Faint)).draw(ctx, room);
        return;
    }
    rows(ctx, (app, open), (body, ui, scoping), (&lines, &labels));
}

/// The line the search is typed in, led by what it picks.
fn typed(ctx: &mut Ctx, line: Rect, scoping: &Scoping) {
    let step = match scoping.which {
        Picks::Contexts => "context",
        _ => "namespace",
    };
    let query = &scoping.query;
    let typed = (query.text(), query.before(), None);
    stepped_input(ctx, line, Lead::Step(step), typed);
}

/// One line: a heading, a held line behind its check, the rest set in.
fn item<'a>(
    ctx: &Ctx,
    (app, open): (&AppState, &Open),
    held: &crate::views::session::resources::ResourcesUi,
    (at, line, label, lit): (usize, &ScopeLine, &'a str, &'a [std::ops::Range<usize>]),
) -> Row<'a, Target> {
    let pad = ctx.tokens.md;
    let accent = ctx.styles.label(Role::Accent);
    let shown = match line {
        ScopeLine::Heading(_) => {
            return Row::new(pad, label, ctx.styles.small(Role::Faint));
        }
        ScopeLine::Held(context) => Some(!held.hidden_contexts.contains(context)),
        ScopeLine::Namespace(pair) => Some(shows(open, held, pair)),
        ScopeLine::Known(_) => None,
    };
    let Some(shown) = shown else {
        let past = pad + ctx.tokens.icon + ctx.tokens.sm;
        let row = Row::new(past, label, ctx.styles.label(Role::Muted));
        return row.lit(lit, accent).target(Target::ScopeLine(at));
    };
    let (mark, role) = match shown {
        true => (Mark::Ticked, Role::Ok),
        false => (Mark::Unticked, Role::Faint),
    };
    let hue = match line {
        ScopeLine::Held(context) => app.config.cluster(context).map(|one| Role::Hue(one.hue)),
        _ => None,
    };
    Row::new(pad, label, ctx.styles.label(hue.unwrap_or(Role::Text)))
        .lit(lit, accent)
        .mark(mark)
        .tint(ctx.styles.label(role).color)
        .mark_size(ctx.tokens.small)
        .target(Target::ScopeLine(at))
}

/// The lines around the cursor, as many as the panel shows; a held context ends in its ×.
fn rows(
    ctx: &mut Ctx,
    (app, open): (&AppState, &Open),
    (body, ui, scoping): (Rect, &crate::Ui, &Scoping),
    (lines, labels): (&[ScopeLine], &[String]),
) {
    let first_pick = lines
        .iter()
        .position(|one| !matches!(one, ScopeLine::Heading(_)));
    let cursor = scoping.cursor.or(first_pick);
    let first = cursor.unwrap_or_default().saturating_sub(PALETTE_LINES - 1);
    let held = &ui.session.resources;
    let query = scoping.query.text();
    let found: Vec<_> = labels
        .iter()
        .map(|one| groove_types::spans(one, &[query]))
        .collect();
    let items: Vec<Row<'_, Target>> = lines
        .iter()
        .zip(labels)
        .enumerate()
        .skip(first)
        .take(PALETTE_LINES)
        .map(|(at, (line, label))| item(ctx, (app, open), held, (at, line, label, &found[at])))
        .collect();
    glass_list(ctx, body, &items, cursor.map(|at| at - first));
    let slots = column_of(
        body,
        &vec![Spec::default().height(ctx.tokens.glass_row); PALETTE_LINES],
    );
    for (slot, (at, line)) in slots
        .into_iter()
        .zip(lines.iter().enumerate().skip(first).take(PALETTE_LINES))
    {
        if matches!(line, ScopeLine::Held(_)) {
            let mut room = slot.pad(Edges::across(0.0, ctx.tokens.sm * 2.0));
            let close = Button::icon(Mark::Close, 0, Target::ScopeDetach(at), Role::Bad);
            close
                .text(Text::Small)
                .hover(ctx.styles.glass_hover())
                .right(ctx, &mut room, 0.0);
        }
    }
}
