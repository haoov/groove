//! The board: every task, live or not, in three columns beside the rail.

mod attention;
mod column;
pub mod complete;
pub mod filter;
mod header;
pub mod plan;
mod state;
mod timeline;

pub use state::BoardUi;

use groove_controllers::AppState;
use groove_gfx::Rect;

use crate::Ui;
use crate::ctx::Ctx;

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

/// The board: one header line, the three columns, the timeline under them.
pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui) {
    let area = ctx.layout.board;
    ctx.quad(area, ctx.styles.ground());
    let (head, body, band) = regions(&ctx.tokens, app, ui, area);
    let field = header::draw(ctx, head, app, ui);
    let width = (body.w / List::ALL.len() as f32).floor();
    for (at, list) in List::ALL.into_iter().enumerate() {
        let x = body.x + width * at as f32;
        column::draw(ctx, Rect::new(x, body.y, width, body.h), app, ui, list);
    }
    timeline::draw(ctx, band, app, ui);
    header::offers(ctx, field, app, ui);
}

/// What the board is made of: its header, its columns, and the timeline's band.
pub fn regions(
    tokens: &crate::tokens::Tokens,
    app: &AppState,
    ui: &Ui,
    area: Rect,
) -> (Rect, Rect, Rect) {
    let head = Rect::new(area.x, area.y, area.w, tokens.header);
    let band = timeline::tall(tokens, app, ui);
    let body = Rect::new(
        area.x,
        head.bottom(),
        area.w,
        (area.h - head.h - band).max(0.0),
    );
    let under = Rect::new(area.x, body.bottom(), area.w, band);
    (head, body, under)
}
