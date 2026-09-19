//! The sidebar's files tab: what changed in the selected worktree, the common root
//! once and then a group per directory under it.

mod bar;
mod results;
mod rows;
mod tree;

pub(crate) use tree::{Listing, listing, reads_as};

use groove_controllers::AppState;
use groove_controllers::workspace_service::FOUND_MAX;
use groove_gfx::Rect;
use groove_types::FileDiff;

use super::commit;
use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use crate::style::Role;
use crate::widget::{button, elide, hairline, row};

pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui) {
    let rect = ctx.layout.sidebar;
    if rect.is_empty() {
        return;
    }
    let panel = ctx.styles.panel();
    ctx.quad(rect, panel);
    edge(ctx, rect);

    let files = narrowed(app, ui);
    let listing = listing(&files);
    let bar = bar::draw(ctx, rect, ui);
    let grep = ui.session.bar.greps();
    let head = Rect::new(rect.x, bar.bottom(), rect.w, ctx.tokens.header);
    match grep {
        true => found(ctx, head, app.workspace.found.len()),
        false => heading(ctx, head, files.len()),
    }
    let under = ctx.layout.commit;
    let body = Rect::new(rect.x, head.bottom(), rect.w, under.y - head.bottom());
    commit::draw(ctx, app, ui, under);
    if grep {
        return results::draw(ctx, body, app, ui);
    }
    if files.is_empty() {
        let style = ctx.styles.small(Role::Faint);
        let line = Rect::new(body.x, body.y, body.w, ctx.tokens.row);
        return row(ctx, line, ctx.tokens.md, "nothing changed", style);
    }
    let open = app.workspace.opened.as_ref().map(|file| &file.path);
    rows::draw(ctx, body, &listing, open, ui);
}

fn edge(ctx: &mut Ctx, rect: Rect) {
    let (rule, thickness) = (ctx.styles.line(), ctx.tokens.hairline);
    ctx.quad(Rect::new(rect.x, rect.y, thickness, rect.h), rule);
}

/// The changed files the bar's path term leaves, every one of them when it is empty.
pub(crate) fn narrowed<'a>(app: &'a AppState, ui: &Ui) -> Vec<&'a FileDiff> {
    let files: Vec<&FileDiff> = changed(app).iter().collect();
    let query = ui.session.bar.path.text();
    match query.is_empty() {
        true => files,
        false => crate::palette::matching(files, |file| file.path.clone(), query),
    }
}

/// How much the search across the worktree has turned up.
fn found(ctx: &mut Ctx, rect: Rect, count: usize) {
    let style = ctx.styles.heading(Role::Faint);
    let label = match count {
        0 => "NOTHING FOUND".to_string(),
        n if n >= FOUND_MAX => format!("FOUND {n}+"),
        n => format!("FOUND · {n}"),
    };
    row(ctx, rect, ctx.tokens.md, &label, style);
    hairline(ctx, rect, ctx.styles.line());
}

fn heading(ctx: &mut Ctx, rect: Rect, count: usize) {
    let (rule, pad) = (ctx.styles.line(), ctx.tokens.md);
    let style = ctx.styles.heading(Role::Faint);
    let label = match count {
        0 => "FILES".to_string(),
        n => format!("FILES · {n}"),
    };
    row(ctx, rect, pad, &label, style);
    hairline(ctx, rect, rule);
}

/// One word at the end of a row, with a ground of its own under the pointer.
pub(crate) fn acted(ctx: &mut Ctx, line: Rect, label: &str, target: Target, ui: &Ui) -> f32 {
    let style = ctx.styles.small(Role::Muted);
    let on_it = ui.hover.as_ref() == Some(&target);
    let ground = on_it.then(|| ctx.styles.action());
    let box_ = button(ctx, line, label, style, ground);
    ctx.hit(box_, target);
    box_.x
}

/// A question in the row's own place, with its two answers at its end.
pub(crate) fn asking(ctx: &mut Ctx, line: Rect, question: &str, ui: &Ui) {
    ctx.quad(line, ctx.styles.raised());
    let keep = acted(ctx, line, "keep", Target::Keep, ui);
    let gone = Rect::new(line.x, line.y, keep - line.x, line.h);
    let discard = acted(ctx, gone, "discard", Target::Discard, ui);
    let style = ctx.styles.small(Role::Bad);
    let asked = Rect::new(line.x, line.y, discard - line.x, line.h);
    let room = (asked.w - ctx.tokens.md * 2.0).max(0.0);
    let text = elide(ctx, question, &style, room);
    row(ctx, asked, ctx.tokens.md, &text, style);
}

/// The files of the worktree the session points at, never another's.
pub(crate) fn changed(app: &AppState) -> &[FileDiff] {
    let selected = app
        .session
        .selected()
        .and_then(|open| open.selected_worktree())
        .map(|worktree| &worktree.id);
    app.workspace.files_of(selected)
}
