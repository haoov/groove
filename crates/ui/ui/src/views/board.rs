//! The board: every task, live or not, in three columns beside the rail.

mod attention;
mod column;
pub mod complete;
pub mod filter;
mod header;
mod live;
pub mod plan;
mod row;
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
    let bands = bands(&ctx.tokens, app, ui, area);
    let field = header::draw(ctx, bands.header, app, ui);
    let (body, width) = (
        bands.columns,
        (bands.columns.w / List::ALL.len() as f32).floor(),
    );
    for (at, list) in List::ALL.into_iter().enumerate() {
        let x = body.x + width * at as f32;
        column::draw(ctx, Rect::new(x, body.y, width, body.h), app, ui, list);
    }
    timeline::draw(ctx, bands.timeline, app, ui);
    header::offers(ctx, field, app, ui);
}

/// What the board is made of.
pub struct Bands {
    pub header: Rect,
    pub columns: Rect,
    pub timeline: Rect,
}

/// The board's own three bands, top to bottom.
pub fn bands(tokens: &crate::tokens::Tokens, app: &AppState, ui: &Ui, area: Rect) -> Bands {
    let header = Rect::new(area.x, area.y, area.w, tokens.header);
    let tall = timeline::height(tokens, app, ui);
    let columns = Rect::new(
        area.x,
        header.bottom(),
        area.w,
        (area.h - header.h - tall).max(0.0),
    );
    Bands {
        timeline: Rect::new(area.x, columns.bottom(), area.w, tall),
        header,
        columns,
    }
}
