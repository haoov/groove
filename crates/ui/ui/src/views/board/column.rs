//! One column of the board: its heading, then a row per item.

use groove_controllers::AppState;
use groove_gfx::Rect;

use super::List;
use super::row::{self, Line};
use crate::Ui;
use crate::base::ctx::Ctx;
use crate::base::hit::Scroller;
use crate::base::style::Role;
use crate::shape::hairline;
use crate::text::row;
use crate::views::session::worktree_row;
use crate::widgets::scrolled;

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
        .filter(|line| matches!(line, Line::Session(_) | Line::Task(_, _) | Line::Review(_)))
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
    scrolled(
        ctx,
        body,
        (which, ui.offset(which)),
        &rows,
        |(_, tall)| *tall,
        |ctx, rect, (at, _)| one(ctx, rect, app, ui, &lines[*at], closes(lines, *at)),
    );
    if list == List::Next && ui.carried().is_some() {
        ctx.clipped(body, |ctx| super::plan::dragging(ctx, body, app, ui));
    }
}

/// Whether this line ends its item: the next one starts another, or there is none.
fn closes(lines: &[Line<'_>], at: usize) -> bool {
    !matches!(lines.get(at + 1), Some(Line::Worktree(_, _)))
}

fn one(ctx: &mut Ctx, rect: Rect, app: &AppState, ui: &Ui, line: &Line<'_>, closes: bool) {
    match line {
        Line::Session(living) => super::live::session(ctx, rect, app, ui, living),
        Line::Worktree(worktree, delivery) => {
            worktree_row::draw(ctx, rect, worktree, delivery.as_ref())
        }
        Line::Task(at, task) => row::up_next(ctx, rect, app, ui, *at, task),
        Line::Review(mr) => return super::review::item(ctx, rect, ui, mr),
        Line::Divider => return super::plan::divider(ctx, rect),
        Line::Nothing(text) => {
            let style = ctx.styles.small(Role::Faint);
            return row(ctx, rect, ctx.tokens.md, text, style);
        }
    }
    if closes {
        hairline(ctx, rect, ctx.styles.line());
    }
}
