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
    let [line, _] = column_in(
        area,
        [Spec::default().height(ctx.tokens.header), Spec::fill()],
    );
    let field = header::draw(ctx, line, app, ui);
    for list in List::ALL {
        column::draw(ctx, column_of(&ctx.tokens, area, list), app, ui, list);
    }
    header::offers(ctx, field, app, ui);
}

/// What a click on each row of a column aims at, where the row starts, and how tall it stands.
pub fn spans(
    app: &AppState,
    ui: &Ui,
    list: List,
    tokens: &groove_ui_kit::base::tokens::Tokens,
) -> Vec<(crate::hit::Target, f32, f32)> {
    use crate::hit::Target;
    let card = row::card(tokens);
    let even = |targets: Vec<Target>| -> Vec<(Target, f32, f32)> {
        let at = |(n, one): (usize, Target)| (one, n as f32 * card, card);
        targets.into_iter().enumerate().map(at).collect()
    };
    match list {
        List::Live => even(
            live::living(app, ui)
                .into_iter()
                .map(|one| Target::Session(one.session.id.clone()))
                .collect(),
        ),
        List::Review => even(
            review::sorted(app, ui)
                .into_iter()
                .map(|mr| Target::Review(mr.project.clone(), mr.iid))
                .collect(),
        ),
        List::Next => {
            let lines = plan::lines(app, ui);
            let heights = row::heights(tokens, app, &lines);
            let mut top = 0.0;
            let mut out = Vec::new();
            for (line, tall) in lines.iter().zip(heights) {
                if let row::Line::Task(task) = line {
                    out.push((Target::Task(task.short_id.clone()), top, tall));
                }
                top += tall;
            }
            out
        }
    }
}

/// The room one list's column takes under the header.
pub fn column_of(tokens: &groove_ui_kit::base::tokens::Tokens, area: Rect, list: List) -> Rect {
    let body = columns(tokens, area);
    let wide = Spec::default().width((body.w / List::ALL.len() as f32).floor());
    let at = List::ALL.iter().position(|one| *one == list).unwrap_or(0);
    row_in(body, [wide; List::ALL.len()])[at]
}

/// The list whose column stands at `x`.
pub fn list_at(tokens: &groove_ui_kit::base::tokens::Tokens, area: Rect, x: f32) -> Option<List> {
    let within = |list: &List| {
        let column = column_of(tokens, area, *list);
        x >= column.x && x < column.right()
    };
    List::ALL.into_iter().find(within)
}

/// The room the three columns share under the header.
pub fn columns(tokens: &groove_ui_kit::base::tokens::Tokens, area: Rect) -> Rect {
    let [_, rest] = column_in(area, [Spec::default().height(tokens.header), Spec::fill()]);
    rest
}
