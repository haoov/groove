//! One column of the board: its heading, then a row per item.

use groove_controllers::AppState;
use groove_controllers::session_service::Living;
use groove_gfx::Rect;
use groove_types::{Task, Worktree, WorktreeDelivery};

use super::List;
use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::{Scroller, Target};
use crate::mark::Mark;
use crate::style::Role;
use crate::views::session::worktree_row;
use crate::widget::{after_mark, elide, hairline, icon, leading, row};

/// One line of a column: an item, a worktree under an open one, or the plan's divider.
pub(super) enum Line<'a> {
    Session(&'a Living),
    Worktree(&'a Worktree, Option<&'a WorktreeDelivery>),
    /// Its place in the plan, counted from one.
    Task(usize, &'a Task),
    Divider,
    Nothing(&'static str),
}

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
        List::Live => live(app, ui),
        List::Next => super::plan::lines(app, ui),
        List::Review => vec![Line::Nothing("reviews arrive with the MRs")],
    }
}

/// Every session the filter lets through, with its worktrees under it while it is open.
fn live<'a>(app: &'a AppState, ui: &Ui) -> Vec<Line<'a>> {
    let query = ui.board.query();
    let living: Vec<&Living> = app
        .session
        .living
        .iter()
        .filter(|living| query.lets_session(living))
        .collect();
    if living.is_empty() {
        return vec![Line::Nothing(match query.is_empty() {
            true => "nothing here. Ctrl+Shift+N starts an explorer",
            false => "nothing the filter lets through",
        })];
    }
    let mut lines = Vec::new();
    for living in living {
        lines.push(Line::Session(living));
        if !ui.board.is_open(&living.session.id) {
            continue;
        }
        let open = app.session.get(&living.session.id);
        for worktree in &living.worktrees {
            let delivery = open.and_then(|open| {
                open.delivery
                    .iter()
                    .find(|(id, _)| *id == worktree.id)
                    .map(|(_, delivery)| delivery)
            });
            lines.push(Line::Worktree(worktree, delivery));
        }
    }
    lines
}

/// How many items a column holds; its worktrees and its hints are not items.
fn counted(lines: &[Line<'_>]) -> usize {
    lines
        .iter()
        .filter(|line| matches!(line, Line::Session(_) | Line::Task(_, _)))
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
    let height = item(&ctx.tokens);
    let extent = (height * lines.len() as f32 - body.h).max(0.0);
    ctx.scrolls(Scroller::Column(list as u8), extent);
    let scroll = ui.board.scroll(list).min(extent);
    ctx.clipped(body, |ctx| {
        for (at, line) in lines.iter().enumerate() {
            let y = body.y - scroll + height * at as f32;
            if y + height < body.y || y > body.bottom() {
                continue;
            }
            let rect = Rect::new(body.x, y, body.w, height);
            one(ctx, rect, app, ui, line, closes(lines, at));
        }
        if list == List::Next && ui.board.dragging.is_some() {
            super::plan::dragging(ctx, body, ui, scroll);
        }
    });
}

/// How tall one line of a column is.
pub(super) fn item(tokens: &crate::tokens::Tokens) -> f32 {
    tokens.row + tokens.sm
}

/// Whether this line ends its item: the next one starts another, or there is none.
fn closes(lines: &[Line<'_>], at: usize) -> bool {
    !matches!(lines.get(at + 1), Some(Line::Worktree(_, _)))
}

fn one(ctx: &mut Ctx, rect: Rect, app: &AppState, ui: &Ui, line: &Line<'_>, closes: bool) {
    match line {
        Line::Session(living) => session(ctx, rect, app, ui, living),
        Line::Worktree(worktree, delivery) => worktree_row::draw(ctx, rect, worktree, *delivery),
        Line::Task(at, task) => waiting(ctx, rect, ui, *at, task),
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

/// One session: a twisty for its worktrees, its kind, its title, what it holds.
fn session(ctx: &mut Ctx, line: Rect, app: &AppState, ui: &Ui, living: &Living) {
    let id = &living.session.id;
    let target = Target::Session(id.clone());
    if ui.hover.as_ref() == Some(&target) {
        ctx.quad(line, ctx.styles.hover());
    }
    ctx.hit(line, target);
    let kind = twisty(ctx, line, ui, living);
    let box_ = leading(ctx, line, line.x + kind);
    let role = match app.session.get(id).is_some() {
        true => Role::Working,
        false => Role::Ghost,
    };
    icon(ctx, box_, Mark::of_kind(&living.session.kind), role);
    let until = aside(ctx, line, &held(living));
    named(
        ctx,
        line,
        &living.session.title,
        until,
        after_mark(ctx, kind),
    );
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

/// What opens a session's worktrees under it. Returns where the kind icon goes.
fn twisty(ctx: &mut Ctx, line: Rect, ui: &Ui, living: &Living) -> f32 {
    let box_ = leading(ctx, line, line.x + ctx.tokens.sm);
    let turn = match ui.board.is_open(&living.session.id) {
        true => 0,
        false => Mark::RIGHTWARDS,
    };
    ctx.icon(box_, Mark::Down, turn, ctx.styles.color(Role::Ghost));
    ctx.hit(box_, Target::Unfold(living.session.id.clone()));
    after_mark(ctx, ctx.tokens.sm)
}

/// One task waiting: its place in the plan, its title, and what it is worth.
fn waiting(ctx: &mut Ctx, line: Rect, ui: &Ui, at: usize, task: &Task) {
    let target = Target::Task(task.short_id.clone());
    if ui.hover.as_ref() == Some(&target) {
        ctx.quad(line, ctx.styles.hover());
    }
    ctx.hit(line, target);
    let start = place(ctx, line, at, task);
    let until = aside(ctx, line, &worth(task));
    named(ctx, line, &task.title, until, start);
}

/// Where a task sits in the plan, and what a drag takes hold of. Returns the title's x.
fn place(ctx: &mut Ctx, line: Rect, at: usize, task: &Task) -> f32 {
    let style = ctx.styles.code(Role::Ghost);
    let text = at.to_string();
    let room = ctx.measure("00", &style);
    let width = ctx.measure(&text, &style);
    let box_ = Rect::new(line.x + ctx.tokens.md, line.y, room, line.h);
    row(ctx, box_, room - width, &text, style);
    ctx.hit(box_, Target::Place(task.external_id.clone()));
    box_.right() - line.x + ctx.tokens.sm
}

/// The priority and the estimate, as the row's right-hand text.
fn worth(task: &Task) -> String {
    let priority = task.priority.map(|one| one.label().to_string());
    let estimate = task.estimate.map(|hours| format!("{hours}h"));
    [priority, estimate]
        .into_iter()
        .flatten()
        .collect::<Vec<String>>()
        .join(" · ")
}

/// A row's right-hand text. Returns where it starts.
fn aside(ctx: &mut Ctx, line: Rect, text: &str) -> f32 {
    if text.is_empty() {
        return line.right() - ctx.tokens.md;
    }
    let style = ctx.styles.small(Role::Faint);
    let width = ctx.measure(text, &style);
    let at = line.right() - ctx.tokens.md - width;
    row(ctx, Rect::new(at, line.y, width, line.h), 0.0, text, style);
    at
}

/// A row's title, from `start`, cut where its right-hand text begins.
fn named(ctx: &mut Ctx, line: Rect, title: &str, until: f32, start: f32) {
    let style = ctx.styles.label(Role::Text);
    let room = (until - line.x - start - ctx.tokens.sm).max(0.0);
    let text = elide(ctx, title, &style, room);
    row(ctx, line, start, &text, style);
}
