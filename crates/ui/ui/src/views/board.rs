//! The board: every task, live or not, in three columns beside the rail.

mod attention;
mod column;
pub mod complete;
pub mod filter;
mod header;
mod live;
pub mod plan;
pub mod review;
mod row;
mod state;

pub use state::BoardUi;

use groove_controllers::AppState;
use groove_gfx::Rect;

use crate::Ui;
use crate::ctx::Ctx;
use groove_ui_kit::base::ground::Ground;
use groove_ui_kit::layout::{Spec, column_in, row_in};

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
    groove_ui_kit::shape::ground(ctx, area, Ground::Work);
    let field = header::draw(ctx, area.until_y(area.y + ctx.tokens.header), app, ui);
    for list in List::ALL {
        column::draw(ctx, column_of(&ctx.tokens, area, list), app, ui, list);
    }
    header::offers(ctx, field, app, ui);
}

/// The room one list's column takes under the header.
pub fn column_of(tokens: &groove_ui_kit::base::tokens::Tokens, area: Rect, list: List) -> Rect {
    let body = columns(tokens, area);
    let wide = Spec::default().width((body.w / List::ALL.len() as f32).floor());
    let at = List::ALL.iter().position(|one| *one == list).unwrap_or(0);
    row_in(body, [wide; List::ALL.len()])[at]
}

/// The list whose column stands at `x`.
pub fn list_at(area: Rect, x: f32) -> Option<List> {
    let width = (area.w / List::ALL.len() as f32).floor().max(1.0);
    let at = ((x - area.x) / width).floor().max(0.0) as usize;
    List::ALL.get(at).copied()
}

/// The room the three columns share under the header.
pub fn columns(tokens: &groove_ui_kit::base::tokens::Tokens, area: Rect) -> Rect {
    let [_, rest] = column_in(area, [Spec::default().height(tokens.header), Spec::fill()]);
    rest
}
