//! The board: every task, live or not, in three columns beside the rail.

mod attention;
mod column;
pub mod complete;
pub mod filter;
mod header;
mod live;
pub mod plan;
mod review;
mod row;
mod state;

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

/// The board: one header line, the three columns under it.
pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui) {
    let area = ctx.app.layout.board;
    ctx.quad(area, ctx.styles.ground());
    let mut body = area;
    let field = header::draw(ctx, body.take_top(ctx.tokens.header), app, ui);
    let width = (body.w / List::ALL.len() as f32).floor();
    for list in List::ALL {
        column::draw(ctx, body.take_left(width), app, ui, list);
    }
    header::offers(ctx, field, app, ui);
}

/// The room the three columns share under the header.
pub fn columns(tokens: &groove_ui_kit::base::tokens::Tokens, area: Rect) -> Rect {
    let mut rest = area;
    rest.take_top(tokens.header);
    rest
}
