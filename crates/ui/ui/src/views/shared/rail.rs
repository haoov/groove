use groove_controllers::AppState;
use groove_gfx::Rect;

use super::{rail_item, status};
use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Scroller;
use crate::mark::Mark;
use crate::style::Role;
use crate::widget::{Row, after_mark, hairline, icon, leading, list, row};

/// What the rail remembers between frames.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct RailUi {
    /// How far the rows are scrolled, in pixels.
    pub scroll: f32,
}

/// The opened sessions, in the order opened. The Board row above, the footer below.
pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui) {
    let rect = ctx.layout.rail;
    let panel = ctx.styles.panel();
    ctx.quad(rect, panel);
    edge(ctx, rect);

    let width = rect.w - ctx.tokens.hairline;
    let board = Rect::new(0.0, 0.0, width, ctx.tokens.header);
    let foot = Rect::new(0.0, rect.h - ctx.tokens.row, width, ctx.tokens.row);
    board_row(ctx, board);
    let rows = Rect::new(0.0, board.bottom(), width, foot.y - board.bottom());
    items(ctx, app, ui, rows);
    footer(ctx, app, foot);
}

fn edge(ctx: &mut Ctx, rect: Rect) {
    let (rule, thickness) = (ctx.styles.line(), ctx.tokens.hairline);
    ctx.quad(
        Rect::new(rect.right() - thickness, rect.y, thickness, rect.h),
        rule,
    );
}

/// The board. It carries the attention count when it is not zero.
fn board_row(ctx: &mut Ctx, rect: Rect) {
    let (rule, style) = (ctx.styles.line(), ctx.styles.label(Role::Text));
    let box_ = leading(ctx, rect, ctx.tokens.md);
    icon(ctx, box_, Mark::Board, Role::Faint);
    row(ctx, rect, after_mark(ctx, ctx.tokens.md), "Board", style);
    hairline(ctx, rect, rule);
}

/// One item per open session, scrolled and clipped to `area`.
fn items(ctx: &mut Ctx, app: &AppState, ui: &Ui, area: Rect) {
    let height = rail_item::height(ctx);
    let content = height * app.session.open.len() as f32;
    let extent = (content - area.h).max(0.0);
    ctx.scrolls(Scroller::Rail, extent);
    let scroll = ui.rail.scroll.min(extent);
    ctx.clipped(area, |ctx| {
        let mut y = area.y - scroll;
        for open in &app.session.open {
            rail_item::draw(ctx, app, ui, Rect::new(area.x, y, area.w, height), open);
            y += height;
        }
    });
}

/// The jobs in flight, then settings.
fn footer(ctx: &mut Ctx, app: &AppState, rect: Rect) {
    let rule = ctx.styles.line();
    status::draw(ctx, app, rect.y);
    hairline(
        ctx,
        Rect::new(rect.x, rect.y - rect.h, rect.w, rect.h),
        rule,
    );
    let style = ctx.styles.small(Role::Faint);
    let settings = Row::new(ctx.tokens.md, "settings", style).mark(Mark::Settings);
    list(ctx, rect, &[settings], None);
}
