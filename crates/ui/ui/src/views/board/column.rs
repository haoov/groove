//! One column of the board: its heading, then a row per item.

use groove_controllers::AppState;
use groove_gfx::Rect;

use super::List;
use super::row::{self, Line};
use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Scroller;
use crate::style::Role;
use crate::views::session::worktree_row;
use crate::widget::{hairline, row};

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
    let extent = (heights.iter().sum::<f32>() - body.h).max(0.0);
    ctx.scrolls(Scroller::Column(list as u8), extent);
    let scroll = ui.board.scroll(list).min(extent);
    ctx.clipped(body, |ctx| {
        let mut y = body.y - scroll;
        for (at, line) in lines.iter().enumerate() {
            let height = heights[at];
            if y + height >= body.y && y <= body.bottom() {
                let rect = Rect::new(body.x, y, body.w, height);
                one(ctx, rect, app, ui, line, closes(lines, at));
            }
            y += height;
        }
        if list == List::Next && ui.board.dragging.is_some() {
            super::plan::dragging(ctx, body, app, ui);
        }
    });
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
