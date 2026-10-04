//! One column of the board: its heading, then a row per item.

use groove_controllers::AppState;
use groove_gfx::Rect;

use super::List;
use super::row::{self, Line};
use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Scroller;
use crate::offsets::listed;
use groove_ui_kit::base::ground::Ground;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::hairline;
use groove_ui_kit::text::row;

pub(super) fn draw(ctx: &mut Ctx, area: Rect, app: &AppState, ui: &Ui, list: List) {
    let head = Rect::new(area.x, area.y, area.w, ctx.tokens.header);
    edge(ctx, area);
    let body = Rect::new(area.x, head.bottom(), area.w, area.h - head.h);
    match list {
        List::Review => {
            let asked = super::review::sorted(app, ui);
            heading(ctx, head, list, asked.len());
            return super::review::draw(ctx, body, ui, &asked);
        }
        List::Live => {
            let living = super::live::living(app, ui);
            heading(ctx, head, list, living.len());
            if living.is_empty() {
                return nothing(ctx, body, super::live::empty(ui));
            }
            return super::live::draw(ctx, body, app, ui, &living);
        }
        List::Next => {}
    }
    let lines = lines(app, ui, list);
    heading(ctx, head, list, counted(&lines));
    rows(ctx, body, app, ui, list, &lines);
}

/// What a column holds, top to bottom.
fn lines<'a>(app: &'a AppState, ui: &Ui, list: List) -> Vec<Line<'a>> {
    match list {
        List::Next => super::plan::lines(app, ui),
        List::Live | List::Review => Vec::new(),
    }
}

/// How many items a column holds; its worktrees and its hints are not items.
fn counted(lines: &[Line<'_>]) -> usize {
    lines
        .iter()
        .filter(|line| matches!(line, Line::Task(_)))
        .count()
}

fn heading(ctx: &mut Ctx, line: Rect, list: List, count: usize) {
    groove_ui_kit::shape::ground(ctx, line, Ground::Band);
    hairline(ctx, line, ctx.styles.line());
    groove_ui_kit::widgets::Heading::new(list.name())
        .count(count)
        .draw(ctx, line);
}

/// The rule that separates this column from the one before it.
fn edge(ctx: &mut Ctx, area: Rect) {
    groove_ui_kit::shape::side_rule(ctx, area, area.x, ctx.styles.line());
}

/// The lines a column has room for, scrolled and clipped to it.
fn rows(ctx: &mut Ctx, body: Rect, app: &AppState, ui: &Ui, list: List, lines: &[Line<'_>]) {
    let heights = row::heights(&ctx.tokens, app, lines);
    let rows: Vec<(usize, f32)> = heights.into_iter().enumerate().collect();
    let which = Scroller::Column(list as u8);
    listed(
        ctx,
        body,
        (which, ui.offset(which)),
        &rows,
        |(_, tall)| *tall,
        |ctx, rect, (at, _)| one(ctx, rect, app, &lines[*at]),
    );
    if list == List::Next && ui.carried().is_some() {
        ctx.clipped(body, |ctx| super::plan::dragging(ctx, body, app, ui));
    }
}

fn one(ctx: &mut Ctx, rect: Rect, app: &AppState, line: &Line<'_>) {
    match line {
        Line::Task(task) => row::up_next(ctx, rect, app, task),
        Line::Divider => return super::plan::divider(ctx, rect),
    }
    hairline(ctx, rect, ctx.styles.line());
}

/// The one line a column says when it holds nothing.
fn nothing(ctx: &mut Ctx, body: Rect, text: &str) {
    let line = Rect::new(body.x, body.y, body.w, ctx.tokens.row);
    row(
        ctx,
        line,
        ctx.tokens.md,
        text,
        ctx.styles.small(Role::Faint),
    );
}
