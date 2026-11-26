//! The Diff tab's sidebar in the modes layout: the whole change's totals on its head, the strip
//! that picks the list, then the changed files as a tree or flat, the MR's threads or the commits.

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};
use groove_types::FileDiff;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, column_in};
use groove_ui_kit::shape::from_top;
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{
    Heading, changes, flat_row, folder, glass_hoverable, marked_glass_tabs, panel_head, scrollbar,
    solid_pane,
};

use super::super::state::Pane;
use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::{Scroller, Target};

pub(crate) mod tree;

use tree::{Kind, Line};

/// The sidebar in `rect`, its head folding it.
pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui, (rect, shown): (Rect, bool)) {
    solid_pane(ctx, rect);
    let mut room = rect;
    let top = from_top(&mut room, ctx.tokens.bar, 0.0);
    panel_head(ctx, top, !shown);
    let row = flat_row(ctx, top, Target::AsideFold, false);
    let files = super::changed(app);
    let added: u32 = files.iter().map(|one| one.added).sum();
    let deleted: u32 = files.iter().map(|one| one.deleted).sum();
    let mut inner = row.pad(Edges::across(ctx.tokens.md, 0.0));
    changes(ctx, &mut inner, (added, deleted), |ctx, role| {
        ctx.styles.small(role)
    });
    let said = match files.len() {
        1 => "1 file".to_string(),
        many => format!("{many} files"),
    };
    Heading::new("diff")
        .note(&said)
        .fold(shown)
        .within(ctx, row, inner);
    if !shown {
        return;
    }
    let tall = Spec::default().height(ctx.tokens.row + ctx.tokens.sm);
    let [strip, body] = column_in(room, [tall, Spec::fill()]);
    panes(ctx, strip, app, ui);
    let extent = match ui.session.pane {
        Pane::Notes => return super::notes::draw(ctx, body, app, ui),
        Pane::Commits => return super::commits::draw(ctx, body, app, ui),
        Pane::Files => listed(ctx, body, app, ui),
    };
    scrollbar(ctx, body, (ui.offset(Scroller::Files), extent));
}

/// Files, commits, threads: each counted, the one up lit.
fn panes(ctx: &mut Ctx, rect: Rect, app: &AppState, ui: &Ui) {
    let labels: Vec<String> = Pane::ALL
        .iter()
        .map(|pane| {
            let name = match pane {
                Pane::Files => "files",
                Pane::Commits => "commits",
                Pane::Notes => "threads",
            };
            match super::counted(app, *pane) {
                0 => name.to_string(),
                n => format!("{name} · {n}"),
            }
        })
        .collect();
    let shown: Vec<(Option<_>, &str, Target)> = labels
        .iter()
        .zip(Pane::ALL)
        .map(|(label, pane)| (None, label.as_str(), Target::Pane(pane)))
        .collect();
    let at = Pane::ALL.iter().position(|one| *one == ui.session.pane);
    marked_glass_tabs(ctx, rect, &shown, at.unwrap_or(0));
}

/// The changed files, a right click on them switching tree and list; returns how far they scroll.
fn listed(ctx: &mut Ctx, body: Rect, app: &AppState, ui: &Ui) -> f32 {
    let files: Vec<&FileDiff> = super::changed(app).iter().collect();
    ctx.hit(body, Target::Listing);
    if files.is_empty() {
        let line = groove_ui_kit::shape::band(body, 0.0, ctx.tokens.row);
        let line = line.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
        Label::new("nothing changed", ctx.styles.small(Role::Faint)).draw(ctx, line);
        return 0.0;
    }
    let lines = tree::lines(&files, ui.session.flat, &ui.session.closed);
    let at = (Scroller::Files, ui.offset(Scroller::Files));
    let height = ctx.tokens.row;
    crate::offsets::listed(
        ctx,
        body,
        at,
        &lines,
        |_| height,
        |ctx, line, one| drawn(ctx, line, app, one),
    )
}

fn drawn(ctx: &mut Ctx, line: Rect, app: &AppState, one: &Line) {
    let indent = ctx.tokens.md + ctx.tokens.md * one.depth as f32;
    let mut room = line.pad(Edges::across(indent, ctx.tokens.md));
    match &one.kind {
        Kind::Dir(open, count) => {
            glass_hoverable(ctx, line, Target::Group(one.path.clone()));
            folder(ctx, &mut room, *open, Role::Faint);
            if !open {
                let said = match count {
                    1 => "1 file".to_string(),
                    many => format!("{many} files"),
                };
                Label::new(&said, ctx.styles.small(Role::Faint)).right(ctx, &mut room, 0.0);
            }
            Label::new(&one.name, ctx.styles.body(Role::Muted)).draw(ctx, room);
        }
        Kind::File(file, dir) => {
            let shown = app.workspace.changes.focused() == Some(file.path.as_str());
            if shown {
                groove_ui_kit::widgets::glass_chosen(ctx, line);
            }
            glass_hoverable(ctx, line, Target::File(file.path.clone()));
            let read = crate::views::session::diff::is_read(app, &file.path);
            file_row(ctx, room, (file, &one.name, dir.as_deref()), read);
        }
    }
}

/// A file's name and its folder from the left; its counts and its git letter at the right.
fn file_row(ctx: &mut Ctx, mut room: Rect, (file, name, dir): Named, read: bool) {
    let dim = |role: Role| if read { Role::Ghost } else { role };
    let letter = file.status.letter().to_string();
    let style = ctx.styles.small(dim(super::rows::status_role(file.status)));
    Label::new(&letter, style).right(ctx, &mut room, 0.0);
    changes(ctx, &mut room, (file.added, file.deleted), |ctx, role| {
        ctx.styles.small(dim(role))
    });
    let style = ctx.styles.body(dim(Role::Text));
    Label::new(name, style).left(ctx, &mut room, ctx.tokens.sm);
    if let Some(dir) = dir {
        Label::new(dir, ctx.styles.small(Role::Ghost)).draw(ctx, room);
    }
}

/// A changed file, the name its row reads as, and its folder in the flat list.
type Named<'a> = (&'a FileDiff, &'a str, Option<&'a str>);

/// The focus the change needs: the chosen file, else the list's first; none with modes off.
pub(crate) fn focused(app: &AppState, ui: &Ui) -> Option<groove_controllers::Command> {
    let now = app.workspace.changes.focused();
    let files: Vec<&FileDiff> = super::changed(app).iter().collect();
    let kept = now.filter(|path| files.iter().any(|one| one.path == *path));
    let first = || {
        tree::order(&files, ui.session.flat)
            .first()
            .map(|one| one.path.as_str())
    };
    let path = match ui.modes {
        true => kept.or_else(first),
        false => None,
    };
    let changes = &app.workspace.changes;
    let lacks = path.is_none_or(|one| changes.get(one).is_none());
    if path == now || (path.is_some() && lacks) {
        return None;
    }
    let path = path.map(str::to_string);
    let focus = groove_controllers::workspace::Command::Focus { path };
    Some(groove_controllers::Command::Workspace(focus))
}

/// Whether the middle is the modes layout's Diff.
pub(crate) fn diffing(ui: &Ui) -> bool {
    ui.modes && ui.session.mode == crate::views::session::Mode::Diff
}
