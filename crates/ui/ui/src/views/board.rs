//! The board: every task, live or not, in three columns beside the rail.

mod attention;
mod column;
mod live;
pub mod plan;
pub mod review;
mod row;
mod state;

pub use state::BoardUi;

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};

use crate::Ui;
use crate::ctx::Ctx;
use groove_ui_kit::base::ground::Ground;
use groove_ui_kit::base::tokens::Tokens;
use groove_ui_kit::layout::{Spec, row_in};

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

/// The board: its three columns, clear of the bar's room above and below as the agent is.
pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui) {
    let (board, area) = (ctx.app.layout.board, ctx.app.layout.lists);
    groove_ui_kit::widgets::ground_under(ctx, board, Ground::Deep);
    groove_ui_kit::widgets::ground_grid(ctx, board);
    for list in List::ALL {
        let column = column_of(area, list);
        let column = column.pad(Edges::across(ctx.tokens.sm, ctx.tokens.sm));
        column::draw(ctx, column, app, ui, list);
    }
}

/// What a click on each row of a column aims at, where the row starts, and how tall it stands.
pub fn spans(
    app: &AppState,
    ui: &Ui,
    list: List,
    tokens: &Tokens,
) -> Vec<(crate::hit::Target, f32, f32)> {
    use crate::hit::Target;
    let card = row::card(tokens);
    let even = |targets: Vec<Target>| -> Vec<(Target, f32, f32)> {
        let at = |(n, one): (usize, Target)| (one, n as f32 * card, card);
        targets.into_iter().enumerate().map(at).collect()
    };
    match list {
        List::Live => even(
            live::living(app)
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
            let lines = plan::lines(app);
            let heights = row::heights(tokens, app, &lines);
            let mut top = 0.0;
            let mut out = Vec::new();
            for (line, tall) in lines.iter().zip(heights) {
                let row::Line::Task(task) = line;
                out.push((Target::Task(task.short_id.clone()), top, tall));
                top += tall;
            }
            out
        }
    }
}

/// The room one list's column takes across `body`, the board's columns.
pub fn column_of(body: Rect, list: List) -> Rect {
    let wide = Spec::default().width((body.w / List::ALL.len() as f32).floor());
    let at = List::ALL.iter().position(|one| *one == list).unwrap_or(0);
    row_in(body, [wide; List::ALL.len()])[at]
}

/// The list whose column stands at `x`.
pub fn list_at(area: Rect, x: f32) -> Option<List> {
    let within = |list: &List| {
        let column = column_of(area, *list);
        x >= column.x && x < column.right()
    };
    List::ALL.into_iter().find(within)
}
