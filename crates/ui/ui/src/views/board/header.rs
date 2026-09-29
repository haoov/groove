//! The board's header: the filter it is narrowed by, and what starts a task.

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};

use super::complete;
use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::{Panel, hairline};
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{Button, Row, Search, Text, list};

const PLACEHOLDER: &str = "filter — status:, priority:, project:, provider:, kind:, repo:";
const NEW: &str = "+ explorer";

/// The header line. Returns where the filter was drawn.
pub(super) fn draw(ctx: &mut Ctx, line: Rect, app: &AppState, ui: &Ui) -> Rect {
    ctx.quad(line, ctx.styles.ground());
    hairline(ctx, line, ctx.styles.line());
    let mut room = line.until(new_task(ctx, line));
    if app.task.reading {
        room.take_right(ctx.tokens.md);
        let style = ctx.styles.small(Role::Faint);
        Label::new("reading…", style).right(ctx, &mut room, 0.0);
    }
    filter(ctx, room, ui);
    room
}

/// What the filter holds, after the mark it carries.
fn filter(ctx: &mut Ctx, field: Rect, ui: &Ui) {
    let search = Search::new(&ui.board.filter, Target::Filter, ui.board.typing);
    search.hint(PLACEHOLDER).draw(ctx, field);
}

/// The rows the field offers, over the columns, while the keyboard is in it.
pub(super) fn offers(ctx: &mut Ctx, field: Rect, app: &AppState, ui: &Ui) {
    if !ui.board.typing {
        return;
    }
    let offers = complete::offers(app, ui.board.filter.text());
    let shown = &offers[..offers.len().min(complete::ROWS)];
    if shown.is_empty() {
        return;
    }
    let whole = Rect::new(
        field.x + ctx.tokens.md,
        field.bottom(),
        ctx.tokens.modal / 2.0,
        ctx.tokens.row * shown.len() as f32,
    );
    ctx.layer();
    let (raised, deep) = (ctx.styles.raised(), ctx.styles.deep());
    Panel::default()
        .ground(raised)
        .border(deep)
        .draw(ctx, whole);
    let lit = hovered(ctx).unwrap_or(ui.board.offer);
    let (style, hover) = (ctx.styles.body(Role::Text), ctx.styles.hover());
    let rows: Vec<Row<'_, Target>> = shown
        .iter()
        .enumerate()
        .map(|(at, offer)| {
            let row = Row::new(ctx.tokens.md, offer, style).target(Target::Offer(at));
            Row {
                background: (at == lit).then_some(hover),
                ..row
            }
        })
        .collect();
    list(ctx, whole, &rows, None);
}

/// Which row the pointer stands on, which takes the light from the keyboard's.
fn hovered(ctx: &Ctx) -> Option<usize> {
    match ctx.hover() {
        Some(Target::Offer(at)) => Some(*at),
        _ => None,
    }
}

/// The button that opens an explorer, at the right end. Returns where it starts.
fn new_task(ctx: &mut Ctx, line: Rect) -> f32 {
    let (band, hover) = (ctx.styles.band(), ctx.styles.hover());
    let new = Button::new(NEW, Target::AddTask, Role::Working, band)
        .text(Text::Label)
        .hover(hover);
    let mut room = line.pad(Edges::across(0.0, ctx.tokens.sm));
    new.right(ctx, &mut room, 0.0).x
}
