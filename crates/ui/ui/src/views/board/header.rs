//! The board's header: the filter on glass in the middle, what starts a task beside it.

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};

use super::complete;
use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::base::tokens::Tokens;
use groove_ui_kit::shape::{Panel, band, strip};
use groove_ui_kit::widgets::{Row, Search, glass_button, glass_list, glass_pane};

const PLACEHOLDER: &str = "filter — status:, priority:, project:, provider:, kind:, repo:";

/// How tall the header stands, its gap above included.
pub fn tall(tokens: &Tokens) -> f32 {
    tokens.row + tokens.sm * 2.0 + tokens.sm
}

/// The header across the top of `area`. Returns where the filter was drawn.
pub(super) fn draw(ctx: &mut Ctx, area: Rect, ui: &Ui) -> Rect {
    let sm = ctx.tokens.sm;
    let tall = ctx.tokens.row + sm * 2.0;
    let line = band(area, sm, tall).pad(Edges::across(sm, sm));
    let wide = ctx.tokens.modal.min(line.w - (tall + sm) * 2.0);
    let left = (line.w - wide) / 2.0;
    let pane = strip(line, left, wide);
    let plus = strip(line, left + wide + sm, tall);
    glass_button(ctx, plus, (Mark::Add, Role::Working), Target::AddTask);
    glass_pane(ctx, pane);
    ctx.hit(pane, Target::Filter);
    filter(ctx, pane.pad(Edges::all(sm)), ui)
}

/// The filter, a rounded field across the room left.
fn filter(ctx: &mut Ctx, room: Rect, ui: &Ui) -> Rect {
    let ground = ctx.styles.glass_hover();
    Panel::default()
        .ground(ground)
        .radius(ctx.tokens.sm)
        .draw(ctx, room);
    let search = Search::new(&ui.board.filter, Target::Filter, ui.board.typing);
    search.hint(PLACEHOLDER).glass(true).draw(ctx, room);
    room
}

/// The rows the field offers, over the columns, while the keyboard is in it.
pub(super) fn offers(ctx: &mut Ctx, field: Rect, app: &AppState, ui: &Ui) {
    if !ui.board.typing {
        return;
    }
    let offers = complete::offers(app, ui.board.filter.text());
    let shown = &offers[..offers
        .len()
        .min(groove_ui_kit::base::tokens::SUGGESTED_ROWS)];
    if shown.is_empty() {
        return;
    }
    let from = field.pad(Edges::across(ctx.tokens.md, 0.0));
    let sm = ctx.tokens.sm;
    let rows_high = ctx.tokens.glass_row * shown.len() as f32;
    let size = (ctx.tokens.modal / 2.0, rows_high + sm * 2.0);
    let at = (from.x, from.bottom() + sm);
    let whole = groove_ui_kit::shape::kept_in(at, size, ctx.window);
    ctx.layer();
    glass_pane(ctx, whole);
    let lit = hovered(ctx).unwrap_or(ui.board.offer);
    let style = ctx.styles.body(Role::Text);
    let rows: Vec<Row<'_, Target>> = shown
        .iter()
        .enumerate()
        .map(|(at, offer)| Row::new(ctx.tokens.md, offer, style).target(Target::Offer(at)))
        .collect();
    glass_list(ctx, whole.pad(Edges::xy(0.0, sm)), &rows, Some(lit));
}

/// Which row the pointer stands on, which takes the light from the keyboard's.
fn hovered(ctx: &Ctx) -> Option<usize> {
    match ctx.hover() {
        Some(Target::Offer(at)) => Some(*at),
        _ => None,
    }
}
