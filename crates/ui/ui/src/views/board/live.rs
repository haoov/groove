//! Live: every session on disk, and what each one holds.

use groove_controllers::AppState;
use groove_controllers::session_service::Living;
use groove_gfx::Rect;

use super::List;
use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::{Scroller, Target};
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::widgets::{Cell, Column, Rows, Shown, Table, Width};

/// Every session the filter lets through.
pub(super) fn living<'a>(app: &'a AppState, ui: &Ui) -> Vec<&'a Living> {
    let query = ui.board.query();
    app.session
        .living
        .iter()
        .filter(|living| living.session.kind.routine().is_none())
        .filter(|living| query.lets_session(living, app.task.worked(&living.session)))
        .collect()
}

/// What the column says when it holds nothing.
pub(super) fn empty(ui: &Ui) -> &'static str {
    match ui.board.query().is_empty() {
        true => "nothing here. + explorer starts one",
        false => "nothing the filter lets through",
    }
}

pub(super) fn draw(ctx: &mut Ctx, body: Rect, app: &AppState, ui: &Ui, living: &[&Living]) {
    let columns = [
        Column {
            label: "title",
            width: Width::Fill,
            end: false,
            sort: None,
        },
        Column {
            label: "holds",
            width: Width::Fit("0 repos · 00 worktrees"),
            end: true,
            sort: None,
        },
    ];
    let table = Table {
        columns: &columns,
        rows: Rows {
            first: super::row::item(&ctx.tokens),
            under: 0.0,
            ruled: true,
        },
        count: living.len(),
        offset: ui.board.live,
        selected: None,
        sorted: None,
    };
    let extent = table.draw(ctx, body, |at| session(app, living[at]));
    ctx.app
        .hits
        .scrolls(Scroller::Column(List::Live as u8), extent);
}

/// One session: its kind, blue while it is open, its title, what it holds.
fn session<'a>(app: &AppState, living: &'a Living) -> Shown<'a, Target> {
    let role = match app.session.get(&living.session.id).is_some() {
        true => Role::Working,
        false => Role::Ghost,
    };
    let mark = Mark::of_kind(&living.session.kind);
    Shown {
        cells: vec![
            Cell::label(living.session.title.as_str(), Role::Text).mark(mark, 0, role),
            Cell::small(held(living), Role::Faint),
        ],
        under: Vec::new(),
        target: Some(Target::Session(living.session.id.clone())),
    }
}

/// What a session holds, as the row's right-hand text.
fn held(living: &Living) -> String {
    let repos = match living.repos {
        1 => "1 repo".to_string(),
        n => format!("{n} repos"),
    };
    match living.worktrees.len() {
        1 => repos,
        n => format!("{repos} · {n} worktrees"),
    }
}
