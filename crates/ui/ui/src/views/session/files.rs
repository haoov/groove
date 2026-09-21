//! The sidebar's files tab: what changed in the selected worktree, the common root
//! once and then a group per directory under it.

mod bar;
mod explorer;
mod results;
mod rows;
mod tree;

pub(crate) use tree::{Listing, listing, reads_as};

use groove_controllers::AppState;
use groove_controllers::workspace_service::FOUND_MAX;
use groove_gfx::Rect;
use groove_types::FileDiff;

use super::commit;
use super::state::Scope;
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
    let panel = ctx.styles.band();
    ctx.quad(rect, panel);
    edge(ctx, rect);

    let files = narrowed(app, ui);
    let bar = bar::draw(ctx, rect, ui);
    let grep = ui.session.bar.greps();
    let head = Rect::new(rect.x, bar.bottom(), rect.w, ctx.tokens.header);
    match grep {
        true => found(ctx, head, app.workspace.found.len()),
        false => heading(ctx, head, files.len(), ui),
    }
    let under = ctx.layout.commit;
    let body = Rect::new(rect.x, head.bottom(), rect.w, under.y - head.bottom());
    commit::draw(ctx, app, ui, under);
    if grep {
        return results::draw(ctx, body, app, ui);
    }
    let open = app.workspace.opened.as_ref().map(|file| &file.path);
    if browsing(ui) {
        let held = explorer::rows(&app.workspace.paths, &files, &ui.session.opened);
        return match held.is_empty() {
            true => says(ctx, body, empty(app)),
            false => explorer::draw(ctx, body, &held, open, ui),
        };
    }
    if files.is_empty() {
        let said = match ui.session.bar.path.is_empty() {
            true => "nothing changed",
            false => "no file of that name",
        };
        return says(ctx, body, said);
    }
    rows::draw(ctx, body, &listing(&files), open, ui);
}

/// Whether the list is the whole worktree, which a query flattens back to matches.
pub(crate) fn browsing(ui: &Ui) -> bool {
    ui.session.scope == Scope::All && ui.session.bar.path.is_empty()
}

/// Whether the explorer still needs the worktree walked before it can draw a tree.
pub(crate) fn needs_walk(app: &AppState, ui: &Ui) -> bool {
    browsing(ui) && holds(app) && app.workspace.paths.is_empty() && !app.workspace.walking
}

/// Whether a worktree is selected at all.
fn holds(app: &AppState) -> bool {
    app.session
        .selected()
        .and_then(|open| open.selected_worktree())
        .is_some()
}

/// What the tree has to say instead of rows.
fn empty(app: &AppState) -> &'static str {
    match (holds(app), app.workspace.walking) {
        (false, _) => "no worktree to read",
        (true, true) => "reading the worktree…",
        (true, false) => "nothing in this worktree",
    }
}

fn says(ctx: &mut Ctx, body: Rect, text: &str) {
    let style = ctx.styles.small(Role::Faint);
    let line = Rect::new(body.x, body.y, body.w, ctx.tokens.row);
    row(ctx, line, ctx.tokens.md, text, style);
}

fn edge(ctx: &mut Ctx, rect: Rect) {
    let (rule, thickness) = (ctx.styles.line(), ctx.tokens.hairline);
    ctx.quad(Rect::new(rect.x, rect.y, thickness, rect.h), rule);
}

/// How many rows the path term leaves at most, since every one of them is drawn.
pub(crate) const ROWS_MAX: usize = 200;

/// The changed files the path term leaves, then the other worktree files it names.
pub(crate) fn narrowed<'a>(app: &'a AppState, ui: &Ui) -> Vec<&'a FileDiff> {
    let changed: Vec<&FileDiff> = changed(app).iter().collect();
    let query = ui.session.bar.path.text();
    if query.is_empty() {
        return changed;
    }
    let mut left = crate::palette::matching(changed, |file| file.path.clone(), query);
    let shown: std::collections::HashSet<&str> =
        left.iter().map(|file| file.path.as_str()).collect();
    let rest: Vec<&FileDiff> = app
        .workspace
        .paths
        .iter()
        .filter(|file| !shown.contains(file.path.as_str()))
        .collect();
    left.extend(crate::palette::matching(
        rest,
        |file| file.path.clone(),
        query,
    ));
    left.truncate(ROWS_MAX);
    left
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

/// The two scopes, the one in use lit and counted, each a word to click.
fn heading(ctx: &mut Ctx, rect: Rect, count: usize, ui: &Ui) {
    hairline(ctx, rect, ctx.styles.line());
    let mut at = rect.x + ctx.tokens.md;
    for scope in Scope::ALL {
        let here = ui.session.scope == scope;
        let role = match here {
            true => Role::Muted,
            false => Role::Ghost,
        };
        let style = ctx.styles.heading(role);
        let label = match (here, count) {
            (true, n) if n >= ROWS_MAX => format!("{} · {n}+", scope.label().to_uppercase()),
            (true, n) if n > 0 => format!("{} · {n}", scope.label().to_uppercase()),
            _ => scope.label().to_uppercase(),
        };
        let width = ctx.measure(&label, &style);
        let line = Rect::new(at, rect.y, width, rect.h);
        row(ctx, line, 0.0, &label, style);
        ctx.hit(line, Target::Scope(scope));
        at += width + ctx.tokens.md;
    }
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
