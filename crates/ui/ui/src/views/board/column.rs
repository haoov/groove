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
use groove_ui_kit::shape::hairline;
use groove_ui_kit::text::row;

pub(super) fn draw(ctx: &mut Ctx, area: Rect, app: &AppState, ui: &Ui, list: List) {
    let lines = lines(app, ui, list);
    let head = Rect::new(area.x, area.y, area.w, ctx.tokens.header);
    heading(ctx, head, list, counted(&lines));
    edge(ctx, area);
    let body = Rect::new(area.x, head.bottom(), area.w, area.h - head.h);
    rows(ctx, body, app, ui, list, &lines);
}

/// What a column holds, top to bottom.
fn lines<'a>(app: &'a AppState, ui: &Ui, list: List) -> Vec<Line<'a>> {
    match list {
        List::Live => super::live::lines(app, ui),
        List::Next => super::plan::lines(app, ui),
        List::Review => super::review::lines(app, ui),
    }
}

/// How many items a column holds; its worktrees and its hints are not items.
fn counted(lines: &[Line<'_>]) -> usize {
    lines
        .iter()
        .filter(|line| matches!(line, Line::Session(_) | Line::Task(_) | Line::Review(_)))
        .count()
}

fn heading(ctx: &mut Ctx, line: Rect, list: List, count: usize) {
    ctx.quad(line, ctx.styles.band());
    hairline(ctx, line, ctx.styles.line());
    let label = match count {
        0 => list.name().to_string(),
        n => format!("{} · {n}", list.name()),
    };
    row(
        ctx,
        line,
        ctx.tokens.md,
        &label,
        ctx.styles.heading(Role::Faint),
    );
}

/// The rule that separates this column from the one before it.
fn edge(ctx: &mut Ctx, area: Rect) {
    let (rule, thickness) = (ctx.styles.line(), ctx.tokens.hairline);
    ctx.quad(Rect::new(area.x, area.y, thickness, area.h), rule);
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
        Line::Session(living) => super::live::session(ctx, rect, app, living),
        Line::Task(task) => row::up_next(ctx, rect, app, task),
        Line::Review(mr) => super::review::item(ctx, rect, mr),
        Line::Divider => return super::plan::divider(ctx, rect),
        Line::Nothing(text) => {
            let style = ctx.styles.small(Role::Faint);
            return row(ctx, rect, ctx.tokens.md, text, style);
        }
    }
    hairline(ctx, rect, ctx.styles.line());
}
