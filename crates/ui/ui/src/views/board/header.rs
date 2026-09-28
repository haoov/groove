//! The board's header: the filter it is narrowed by, and what starts a task.

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};

use super::complete;
use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::{Panel, hairline, square};
use groove_ui_kit::text::{Label, row};
use groove_ui_kit::widgets::{Button, Text};

const PLACEHOLDER: &str = "filter — status:, priority:, board:, provider:, kind:, repo:";
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
    ctx.hit(field, Target::Filter);
    let size = ctx.tokens.icon;
    let mut room = field.pad(Edges::across(ctx.tokens.md, 0.0));
    let mark = square(room.take_left(size), size);
    room.take_left(ctx.tokens.sm);
    let role = match ui.board.typing {
        true => Role::Text,
        false => Role::Ghost,
    };
    ctx.icon(mark, Mark::Search, 0, ctx.styles.color(role));
    let (text, style) = match (ui.board.typing, ui.board.filter.is_empty()) {
        (true, _) => (ui.board.filter.shown(), ctx.styles.body(Role::Text)),
        (false, true) => (PLACEHOLDER.to_string(), ctx.styles.body(Role::Ghost)),
        (false, false) => (
            ui.board.filter.text().to_string(),
            ctx.styles.body(Role::Text),
        ),
    };
    row(ctx, room, 0.0, &text, style);
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
    let height = ctx.tokens.row;
    let mut whole = Rect::new(
        field.x + ctx.tokens.md,
        field.bottom(),
        ctx.tokens.modal / 2.0,
        height * shown.len() as f32,
    );
    ctx.layer();
    let (raised, deep) = (ctx.styles.raised(), ctx.styles.deep());
    Panel::default()
        .ground(raised)
        .border(deep)
        .draw(ctx, whole);
    let lit = hovered(ctx).unwrap_or(ui.board.offer);
    for (at, offer) in shown.iter().enumerate() {
        let line = whole.take_top(height);
        if at == lit {
            ctx.quad(line, ctx.styles.hover());
        }
        let text = line.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
        Label::new(offer, ctx.styles.body(Role::Text)).draw(ctx, text);
        ctx.hit(line, Target::Offer(at));
    }
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
