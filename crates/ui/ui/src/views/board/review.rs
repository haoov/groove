//! Review: the merge requests the forges ask this user to look at, as a table.

mod order;

pub use order::{By, Order};

use groove_controllers::AppState;
use groove_gfx::Rect;
use groove_types::{ReviewMr, Timestamp};

use super::List;
use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::{Scroller, Target};
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::base::tokens::Tokens;
use groove_ui_kit::text::{ago, row};
use groove_ui_kit::widgets::{Cell, Column, Rows, Shown, Table, Width};

/// Every MR the filter lets through, in the order the table is sorted by.
pub fn sorted<'a>(app: &'a AppState, ui: &Ui) -> Vec<&'a ReviewMr> {
    let query = ui.board.query();
    let mut asked: Vec<&ReviewMr> = app
        .delivery
        .reviews
        .iter()
        .filter(|mr| query.lets_review(mr))
        .collect();
    ui.board.reviews.sort(&mut asked);
    asked
}

pub fn chosen(ui: &Ui, asked: &[&ReviewMr]) -> Option<usize> {
    let (project, iid) = ui.board.chosen.as_ref()?;
    asked
        .iter()
        .position(|mr| &mr.project == project && mr.iid == *iid)
}

pub(super) fn draw(ctx: &mut Ctx, body: Rect, ui: &Ui, asked: &[&ReviewMr]) {
    if asked.is_empty() {
        let text = match ui.board.query().is_empty() {
            true => "nothing is waiting on you",
            false => "nothing the filter lets through",
        };
        let line = Rect::new(body.x, body.y, body.w, ctx.tokens.row);
        return row(
            ctx,
            line,
            ctx.tokens.md,
            text,
            ctx.styles.small(Role::Faint),
        );
    }
    let columns = columns();
    let tokens = ctx.tokens;
    let first = super::row::item(&tokens);
    let table = Table {
        columns: &columns,
        rows: Rows {
            first,
            under: height(&tokens) - first,
            ruled: true,
        },
        count: asked.len(),
        offset: ui.board.review,
        selected: chosen(ui, asked),
        sorted: Some(ui.board.reviews.sorted()),
    };
    let now = ctx.now;
    let extent = table.draw(ctx, body, |at| shown(asked[at], now));
    ctx.app
        .hits
        .scrolls(Scroller::Column(List::Review as u8), extent);
}

pub fn height(tokens: &Tokens) -> f32 {
    super::row::item(tokens) + tokens.line + tokens.xs
}

fn columns() -> [Column<'static, Target>; 2] {
    let sort = |by: By| Some(Target::SortReview(by));
    [
        Column {
            label: "title",
            width: Width::Fill,
            end: false,
            sort: sort(By::Title),
        },
        Column {
            label: "updated",
            width: Width::Fit("00mo"),
            end: true,
            sort: sort(By::Updated),
        },
    ]
}

/// The card: its state, title and age; under them its project, number, author and review.
fn shown(mr: &ReviewMr, now: Timestamp) -> Shown<'_, Target> {
    let named = format!("{}{}{}", mr.project, mr.forge.sigil(), mr.iid);
    let named = match mr.author.is_empty() {
        true => named,
        false => format!("{named} · {}", mr.author),
    };
    let mut under = vec![Cell::small(named, Role::Faint)];
    if let Some(review) = mr.review {
        let (word, role) = crate::components::review_said(review);
        under.push(Cell::small(word, role).badge());
    }
    Shown {
        cells: vec![
            Cell::label(mr.title.as_str(), Role::Text).mark(Mark::Review, 0, state(mr)),
            Cell::small(ago(mr.updated_at.age_at(now)), Role::Faint),
        ],
        under,
        target: Some(Target::Review(mr.project.clone(), mr.iid)),
    }
}

fn state(mr: &ReviewMr) -> Role {
    match (mr.draft, mr.approved) {
        (true, _) => Role::Ghost,
        (false, true) => Role::Ok,
        (false, false) => Role::Attention,
    }
}
