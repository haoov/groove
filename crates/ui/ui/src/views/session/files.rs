//! The sidebar's files tab: the selected worktree's changes, grouped by directory.

mod bar;
mod commits;
pub(crate) mod explorer;
mod heading;
mod notes;
mod results;
mod rows;
mod tree;
pub(crate) mod walked;

pub(crate) use tree::{Listing, listing, reads_as};

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};
use groove_types::FileDiff;

use super::commit;
use super::state::{Pane, Tab};
use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{Button, tabs};

pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui) {
    let rect = ctx.app.layout.sidebar;
    if rect.is_empty() {
        return;
    }
    ctx.quad(rect, ctx.styles.band());
    let edge = Rect {
        w: ctx.tokens.hairline,
        ..rect
    };
    ctx.quad(edge, ctx.styles.line());
    let under = ctx.app.layout.commit;
    if ui.session.commits() && !under.is_empty() {
        commit::draw(ctx, app, ui, under);
    }
    let mut column = rect.until_y(under.y);
    let bar = bar::draw(ctx, rect, ui);
    column.take_top(bar.h);
    if let Some(one) = app.workspace.commit.as_ref() {
        heading::showing(ctx, column.take_top(ctx.tokens.header), one);
    }
    if ui.session.tab == Tab::Files {
        return changed_files(ctx, column, app, ui);
    }
    let strip = column.take_top(ctx.tokens.row + ctx.tokens.sm);
    panes(ctx, strip, app, ui);
    match ui.session.pane {
        Pane::Files => changed_files(ctx, column, app, ui),
        Pane::Commits => commits::draw(ctx, column, app, ui),
        Pane::Notes => notes::draw(ctx, column, app, ui),
    }
}

/// The three lists the sidebar offers, the one up lit and counted.
fn panes(ctx: &mut Ctx, rect: Rect, app: &AppState, ui: &Ui) {
    let labels: Vec<String> = Pane::ALL.iter().map(|pane| labelled(app, *pane)).collect();
    let shown: Vec<(&str, Target)> = labels
        .iter()
        .zip(Pane::ALL)
        .map(|(label, pane)| (label.as_str(), Target::Pane(pane)))
        .collect();
    let at = Pane::ALL
        .iter()
        .position(|pane| *pane == ui.session.pane)
        .unwrap_or(0);
    tabs(ctx, rect, &shown, at);
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
            .delivery
            .shown
            .iter()
            .filter(|one| !one.resolved && notes::listed(one))
            .count(),
    }
}

/// The files that changed, under their own heading, with the commit box below.
fn changed_files(ctx: &mut Ctx, mut column: Rect, app: &AppState, ui: &Ui) {
    let files = narrowed(app, ui);
    let grep = ui.session.bar.greps();
    let moded = ui.session.tab == Tab::Diff && app.workspace.commit.is_none();
    if grep || moded {
        let head = column.take_top(ctx.tokens.header);
        match grep {
            true => heading::found(ctx, head, app.workspace.found.len()),
            false => heading::modes(ctx, head, app.workspace.mode),
        }
    }
    let body = column;
    if grep {
        return results::draw(ctx, body, app, ui);
    }
    let open = app.workspace.active().map(|file| &file.path);
    if browsing(ui) {
        let held = explorer::rows(
            (app.workspace.paths(), app.workspace.paths_stamp()),
            &files,
            ui,
        );
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
    ui.session.tab == Tab::Files && ui.session.bar.path.is_empty()
}

/// Whether the commits list still needs this worktree's commits read.
pub(crate) fn needs_commits(app: &AppState, ui: &Ui) -> bool {
    let selected = app
        .session
        .selected_worktree()
        .map(|worktree| worktree.id.clone());
    ui.session.pane == Pane::Commits && selected.is_some() && app.workspace.logged != selected
}

/// Whether the notes list still needs this session's notes read.
pub(crate) fn needs_notes(app: &AppState, _: &Ui) -> bool {
    let selected = app.session.selected.as_ref();
    selected.is_some_and(|session| app.delivery.notes_of(session).is_none())
}

/// Whether the explorer still needs the worktree walked before it can draw a tree.
pub(crate) fn needs_walk(app: &AppState, ui: &Ui) -> bool {
    browsing(ui) && holds(app) && app.workspace.paths().is_empty() && !app.workspace.walking
}

/// Whether a worktree is selected at all.
fn holds(app: &AppState) -> bool {
    app.session.selected_worktree().is_some()
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
    let line = Rect {
        h: ctx.tokens.row,
        ..body
    };
    let room = line.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
    Label::new(text, ctx.styles.small(Role::Faint)).draw(ctx, room);
}

/// How many rows the path term keeps at most.
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
        .paths()
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

/// One word at the right of `room`, with a ground of its own under the pointer.
pub(crate) fn acted(ctx: &mut Ctx, room: &mut Rect, label: &str, target: Target) {
    let (ground, action) = (ctx.styles.ground(), ctx.styles.action());
    let word = Button::new(label, target, Role::Muted, ground)
        .flat()
        .hover(action);
    word.right(ctx, room, ctx.tokens.sm);
}

/// A question in the row's own place, with its two answers at its end.
pub(crate) fn asking(ctx: &mut Ctx, line: Rect, question: &str) {
    ctx.quad(line, ctx.styles.raised());
    let mut room = line;
    acted(ctx, &mut room, "keep", Target::Keep);
    acted(ctx, &mut room, "discard", Target::Discard);
    let asked = room.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
    Label::new(question, ctx.styles.small(Role::Bad)).draw(ctx, asked);
}

/// The files of the worktree the session points at, never another's.
pub(crate) fn changed(app: &AppState) -> &[FileDiff] {
    let selected = app.session.selected_worktree().map(|worktree| &worktree.id);
    app.workspace.files_of(selected)
}

/// Whether a note of this session stands on a file.
pub(super) fn noted(app: &AppState, path: &str) -> bool {
    app.delivery
        .shown
        .iter()
        .any(|note| note.anchor.as_ref().is_some_and(|one| one.path == path))
}
