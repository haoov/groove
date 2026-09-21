//! The sidebar's files tab: what changed in the selected worktree, the common root
//! once and then a group per directory under it.

mod bar;
mod commits;
pub(crate) mod explorer;
mod notes;
mod results;
mod rows;
mod tree;

pub(crate) use tree::{Listing, listing, reads_as};

use groove_controllers::AppState;
use groove_controllers::workspace_service::FOUND_MAX;
use groove_gfx::Rect;
use groove_types::FileDiff;

use super::commit;
use super::state::{Pane, Scope};
use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use crate::style::Role;
use crate::widget::{button, elide, hairline, row, tabs};

pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui) {
    let rect = ctx.layout.sidebar;
    if rect.is_empty() {
        return;
    }
    ctx.quad(rect, ctx.styles.band());
    edge(ctx, rect);
    let bar = bar::draw(ctx, rect, ui);
    let strip = Rect::new(rect.x, bar.bottom(), rect.w, ctx.tokens.row + ctx.tokens.sm);
    panes(ctx, strip, app, ui);
    match ui.session.pane {
        Pane::Files => changed_files(ctx, rect, strip, app, ui),
        Pane::Commits => {
            let body = showing(ctx, under(rect, strip), app, ui);
            commits::draw(ctx, body, app, ui);
        }
        Pane::Notes => {
            let body = showing(ctx, under(rect, strip), app, ui);
            notes::draw(ctx, body, app, ui);
        }
    }
}

/// The room a list has under the strip.
fn under(rect: Rect, strip: Rect) -> Rect {
    Rect::new(
        rect.x,
        strip.bottom(),
        rect.w,
        (rect.bottom() - strip.bottom()).max(0.0),
    )
}

/// The commit the surface shows, over the list. Returns the room the list keeps.
fn showing(ctx: &mut Ctx, body: Rect, app: &AppState, ui: &Ui) -> Rect {
    if app.workspace.commit.is_none() {
        return body;
    }
    commit::showing(ctx, body, app, ui);
    let taken = ctx.tokens.row;
    Rect::new(body.x, body.y + taken, body.w, (body.h - taken).max(0.0))
}

/// The three lists the sidebar offers, the one up lit and counted.
fn panes(ctx: &mut Ctx, rect: Rect, app: &AppState, ui: &Ui) {
    let labels: Vec<String> = Pane::ALL.iter().map(|pane| labelled(app, *pane)).collect();
    let shown: Vec<&str> = labels.iter().map(String::as_str).collect();
    let at = Pane::ALL
        .iter()
        .position(|pane| *pane == ui.session.pane)
        .unwrap_or(0);
    for (pane, line) in Pane::ALL.iter().zip(tabs(ctx, rect, &shown, at)) {
        ctx.hit(line, Target::Pane(*pane));
    }
}

/// A list's own name, with what it holds when it holds anything.
fn labelled(app: &AppState, pane: Pane) -> String {
    match counted(app, pane) {
        0 => pane.label().to_string(),
        n => format!("{} · {n}", pane.label()),
    }
}

/// What a pane's own name counts beside it.
fn counted(app: &AppState, pane: Pane) -> usize {
    match pane {
        Pane::Files => changed(app).len(),
        Pane::Commits => app.workspace.log.iter().filter(|one| !one.is_base).count(),
        Pane::Notes => app
            .workspace
            .notes
            .iter()
            .filter(|one| !one.resolved)
            .count(),
    }
}

/// The files that changed, under their own heading, with the commit box below.
fn changed_files(ctx: &mut Ctx, rect: Rect, strip: Rect, app: &AppState, ui: &Ui) {
    let files = narrowed(app, ui);
    let grep = ui.session.bar.greps();
    let head = Rect::new(rect.x, strip.bottom(), rect.w, ctx.tokens.header);
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
        let held = explorer::rows(&app.workspace.paths, &files, ui);
        return match held.is_empty() {
            true => says(ctx, body, empty(app)),
            false => explorer::draw(ctx, body, app, &held, open, ui),
        };
    }
    if files.is_empty() {
        let said = match ui.session.bar.path.is_empty() {
            true => "nothing changed",
            false => "no file of that name",
        };
        return says(ctx, body, said);
    }
    rows::draw(ctx, body, app, &listing(&files), open, ui);
}

/// Whether the list is the whole worktree, which a query flattens back to matches.
pub(crate) fn browsing(ui: &Ui) -> bool {
    ui.session.scope == Scope::All && ui.session.bar.path.is_empty()
}

/// Whether the commits list still needs this worktree's commits read.
pub(crate) fn needs_commits(app: &AppState, ui: &Ui) -> bool {
    let selected = app
        .session
        .selected()
        .and_then(|open| open.selected_worktree())
        .map(|worktree| worktree.id.clone());
    ui.session.pane == Pane::Commits && selected.is_some() && app.workspace.logged != selected
}

/// Whether the notes list still needs this session's notes read.
pub(crate) fn needs_notes(app: &AppState, ui: &Ui) -> bool {
    ui.session.pane == Pane::Notes
        && app.session.selected.is_some()
        && app.workspace.noted != app.session.selected
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

/// The two scopes, the one in use lit and counted, each a tab to click.
fn heading(ctx: &mut Ctx, rect: Rect, count: usize, ui: &Ui) {
    let labels: Vec<String> = Scope::ALL
        .iter()
        .map(|scope| scoped(*scope, count, ui.session.scope == *scope))
        .collect();
    let shown: Vec<&str> = labels.iter().map(String::as_str).collect();
    let at = Scope::ALL
        .iter()
        .position(|scope| *scope == ui.session.scope)
        .unwrap_or(0);
    for (scope, line) in Scope::ALL.iter().zip(tabs(ctx, rect, &shown, at)) {
        ctx.hit(line, Target::Scope(*scope));
    }
}

/// A scope's own name, with what the list holds under it.
fn scoped(scope: Scope, count: usize, here: bool) -> String {
    match (here, count) {
        (true, n) if n >= ROWS_MAX => format!("{} · {n}+", scope.label()),
        (true, n) if n > 0 => format!("{} · {n}", scope.label()),
        _ => scope.label().to_string(),
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

/// Whether a note of this session stands on a file.
pub(super) fn noted(app: &AppState, path: &str) -> bool {
    app.workspace
        .notes
        .iter()
        .any(|note| note.anchor.as_ref().is_some_and(|one| one.path == path))
}
