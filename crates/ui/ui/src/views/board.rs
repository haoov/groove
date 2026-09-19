//! The board: every task, live or not, in three columns beside the rail.

mod column;
mod state;

pub use state::BoardUi;

use groove_controllers::AppState;
use groove_gfx::Rect;

use crate::Ui;
use crate::ctx::Ctx;
use crate::style::Role;
use crate::widget::{hairline, row};

/// Which list a column holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum List {
    Live,
    Next,
    Review,
}

impl List {
    pub const ALL: [List; 3] = [List::Live, List::Next, List::Review];

    pub fn name(self) -> &'static str {
        match self {
            List::Live => "LIVE",
            List::Next => "UP NEXT",
            List::Review => "REVIEW",
        }
    }
}

/// The board: one header line, then the three columns under it.
pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui) {
    let area = ctx.layout.board;
    ctx.quad(area, ctx.styles.ground());
    let head = Rect::new(area.x, area.y, area.w, ctx.tokens.header);
    heading(ctx, head, app);
    let body = Rect::new(area.x, head.bottom(), area.w, area.h - head.h);
    let width = (body.w / List::ALL.len() as f32).floor();
    for (at, list) in List::ALL.into_iter().enumerate() {
        let x = body.x + width * at as f32;
        column::draw(ctx, Rect::new(x, body.y, width, body.h), app, ui, list);
    }
}

/// What the board is, and whether it is still reading.
fn heading(ctx: &mut Ctx, line: Rect, app: &AppState) {
    ctx.quad(line, ctx.styles.panel());
    hairline(ctx, line, ctx.styles.line());
    row(
        ctx,
        line,
        ctx.tokens.md,
        "Board",
        ctx.styles.title(Role::Text),
    );
    if !app.task.reading {
        return;
    }
    let style = ctx.styles.small(Role::Faint);
    let width = ctx.measure("reading…", &style);
    let at = line.right() - ctx.tokens.md - width;
    row(
        ctx,
        Rect::new(at, line.y, width, line.h),
        0.0,
        "reading…",
        style,
    );
}
