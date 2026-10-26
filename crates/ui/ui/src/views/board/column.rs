//! One column of the board: its heading, then a row per item.

use groove_controllers::AppState;
use groove_gfx::Rect;

use super::List;
use super::row::{self, Line};
use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Scroller;
use crate::offsets::listed;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, column_in};
use groove_ui_kit::text::row;

pub(super) fn draw(ctx: &mut Ctx, area: Rect, app: &AppState, ui: &Ui, list: List) {
    let [head, body] = column_in(
        area,
        [Spec::default().height(ctx.tokens.header), Spec::fill()],
    );
    match list {
        List::Review => {
            let asked = super::review::sorted(app, ui);
            heading(ctx, head, list, asked.len());
            super::review::draw(ctx, body, ui, &asked);
        }
        List::Live => {
            let living = super::live::living(app, ui);
            heading(ctx, head, list, living.len());
            match living.is_empty() {
                true => nothing(ctx, body, super::live::empty(ui)),
                false => super::live::draw(ctx, body, app, ui, &living),
            }
        }
        List::Next => {
            let lines = super::plan::lines(app, ui);
            heading(ctx, head, list, counted(&lines));
            rows(ctx, body, app, ui, &lines);
        }
    }
}

/// How many items a column holds.
fn counted(lines: &[Line<'_>]) -> usize {
    lines.len()
}

fn heading(ctx: &mut Ctx, line: Rect, list: List, count: usize) {
    groove_ui_kit::widgets::Heading::new(list.name())
        .count(count)
        .draw(ctx, line);
}

/// The lines a column has room for, scrolled and clipped to it.
fn rows(ctx: &mut Ctx, body: Rect, app: &AppState, ui: &Ui, lines: &[Line<'_>]) {
    let heights = row::heights(&ctx.tokens, app, lines);
    let rows: Vec<(usize, f32)> = heights.into_iter().enumerate().collect();
    let which = Scroller::Column(List::Next as u8);
    listed(
        ctx,
        body,
        (which, ui.offset(which)),
        &rows,
        |(_, tall)| *tall,
        |ctx, rect, (at, _)| one(ctx, rect, app, &lines[*at]),
    );
    if ui.carried().is_some() {
        ctx.clipped(body, |ctx| super::plan::dragging(ctx, body, app, ui));
    }
}

fn one(ctx: &mut Ctx, rect: Rect, app: &AppState, line: &Line<'_>) {
    let Line::Task(task) = line;
    row::up_next(ctx, rect, app, task);
}

/// The one line a column says when it holds nothing.
fn nothing(ctx: &mut Ctx, body: Rect, text: &str) {
    let [line, _] = column_in(body, [Spec::default().height(ctx.tokens.row), Spec::fill()]);
    row(
        ctx,
        line,
        ctx.tokens.md,
        text,
        ctx.styles.small(Role::Faint),
    );
}
