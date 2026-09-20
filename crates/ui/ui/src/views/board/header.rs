//! The board's header: the filter it is narrowed by, and what starts a task.

use groove_controllers::AppState;
use groove_gfx::Rect;

use super::complete;
use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use crate::mark::Mark;
use crate::style::Role;
use crate::widget::{button, hairline, row};

const PLACEHOLDER: &str = "filter — status:, priority:, board:, kind:, repo:";
const NEW: &str = "+ explorer";

/// The header line. Returns where the filter was drawn.
pub(super) fn draw(ctx: &mut Ctx, line: Rect, app: &AppState, ui: &Ui) -> Rect {
    ctx.quad(line, ctx.styles.ground());
    hairline(ctx, line, ctx.styles.line());
    let until = new_task(ctx, line, ui);
    let until = reading(ctx, line, app, until);
    let field = Rect::new(line.x, line.y, until - line.x, line.h);
    filter(ctx, field, ui);
    field
}

/// What the filter holds, after the mark it carries.
fn filter(ctx: &mut Ctx, field: Rect, ui: &Ui) {
    ctx.hit(field, Target::Filter);
    let size = ctx.tokens.icon;
    let box_ = Rect::new(
        field.x + ctx.tokens.md,
        field.y + (field.h - size) / 2.0,
        size,
        size,
    );
    let role = match ui.board.typing {
        true => Role::Text,
        false => Role::Ghost,
    };
    ctx.icon(box_, Mark::Search, 0, ctx.styles.color(role));
    let at = ctx.tokens.md + size + ctx.tokens.sm;
    let (text, style) = match (ui.board.typing, ui.board.filter.is_empty()) {
        (true, _) => (ui.board.filter.shown(), ctx.styles.body(Role::Text)),
        (false, true) => (PLACEHOLDER.to_string(), ctx.styles.body(Role::Ghost)),
        (false, false) => (
            ui.board.filter.text().to_string(),
            ctx.styles.body(Role::Text),
        ),
    };
    row(ctx, field, at, &text, style);
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
    let width = ctx.tokens.modal / 2.0;
    let whole = Rect::new(
        field.x + ctx.tokens.md,
        field.bottom(),
        width,
        height * shown.len() as f32,
    );
    ctx.layer();
    ctx.quad(whole, ctx.styles.raised());
    ctx.border(whole, ctx.styles.deep());
    let lit = hovered(ui).unwrap_or(ui.board.offer);
    for (at, offer) in shown.iter().enumerate() {
        let line = Rect::new(whole.x, whole.y + height * at as f32, whole.w, height);
        if at == lit {
            ctx.quad(line, ctx.styles.hover());
        }
        row(ctx, line, ctx.tokens.md, offer, ctx.styles.body(Role::Text));
        ctx.hit(line, Target::Offer(at));
    }
}

/// Which row the pointer stands on, which takes the light from the keyboard's.
fn hovered(ui: &Ui) -> Option<usize> {
    match ui.hover {
        Some(Target::Offer(at)) => Some(at),
        _ => None,
    }
}

/// The button that opens an explorer, at the right end. Returns where it starts.
fn new_task(ctx: &mut Ctx, line: Rect, ui: &Ui) -> f32 {
    let style = ctx.styles.label(Role::Working);
    let ground = match ui.hover.as_ref() == Some(&Target::AddTask) {
        true => ctx.styles.hover(),
        false => ctx.styles.band(),
    };
    let box_ = button(ctx, line, NEW, style, Some(ground));
    ctx.hit(box_, Target::AddTask);
    box_.x
}

/// Whether a read of the sources is out, left of the button.
fn reading(ctx: &mut Ctx, line: Rect, app: &AppState, until: f32) -> f32 {
    if !app.task.reading {
        return until;
    }
    let style = ctx.styles.small(Role::Faint);
    let width = ctx.measure("reading…", &style);
    let at = until - ctx.tokens.md - width;
    row(
        ctx,
        Rect::new(at, line.y, width, line.h),
        0.0,
        "reading…",
        style,
    );
    at
}
