//! The Files mode's sidebar: the files open, then the worktree as a tree.

use groove_controllers::AppState;
use groove_controllers::workspace_service::Opened;
use groove_gfx::{Edges, Rect};
use groove_types::FileDiff;
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{Button, Heading, cap, glass_chosen, glass_hoverable};

use super::super::explorer::{self, Row};
use crate::ctx::Ctx;
use crate::hit::{Scroller, Target};
use crate::{Losing, Ui};

/// One row of the sidebar, as drawn and as the arrows walk it.
pub(crate) enum Line<'a> {
    Tree(Row<'a>),
    /// What the tree says while it has no row.
    Said(&'static str),
    /// `OPEN · 2` over the files open, `WORKTREE` over the tree once they stand above it.
    Head(&'static str, usize),
    Open(&'a Opened),
}

impl Line<'_> {
    /// What a click on the row does, if anything.
    pub(crate) fn target(&self) -> Option<Target> {
        match self {
            Line::Tree(row) if row.dir => Some(Target::Dir(row.path.clone())),
            Line::Tree(row) => Some(Target::File(row.path.clone())),
            Line::Open(one) => Some(Target::OpenTab(one.path.clone())),
            Line::Said(_) | Line::Head(..) => None,
        }
    }
}

/// `FILES · groove`: the worktree's repo as its note.
pub fn head(ctx: &mut Ctx, app: &AppState, row: Rect, shown: bool) {
    let worktree = app.session.selected_worktree();
    let repo = worktree.map(|one| crate::views::name_of(one.repo.as_str()).to_string());
    let heading = Heading::new("files").fold(shown);
    match &repo {
        Some(repo) => heading.note(repo).draw(ctx, row),
        None => heading.draw(ctx, row),
    }
}

/// The open files' rows, then the tree's.
pub(crate) fn lines<'a>(app: &'a AppState, ui: &Ui) -> Vec<Line<'a>> {
    let mut out: Vec<Line<'a>> = Vec::new();
    let open = app.workspace.buffers().map(|one| one.all());
    let open = open.unwrap_or_default();
    if !open.is_empty() {
        out.push(Line::Head("open", open.len()));
        out.extend(open.iter().map(Line::Open));
        out.push(Line::Head("worktree", 0));
    }
    let changed: Vec<&FileDiff> = super::super::changed(app).iter().collect();
    let paths = (app.workspace.paths(), app.workspace.paths_stamp());
    let tree = explorer::rows(paths, &changed, ui);
    match tree.is_empty() {
        true => out.push(Line::Said(super::super::empty(app))),
        false => out.extend(tree.into_iter().map(Line::Tree)),
    }
    out
}

/// The rows in `room`, scrolled; returns how far they scroll.
pub fn body(ctx: &mut Ctx, app: &AppState, ui: &Ui, room: Rect) -> f32 {
    let lines = lines(app, ui);
    let open = app.workspace.active().map(|one| &one.path);
    let at = (Scroller::Files, ui.offset(Scroller::Files));
    let height = ctx.tokens.row;
    crate::offsets::listed(
        ctx,
        room,
        at,
        &lines,
        |_| height,
        |ctx, line, one| match one {
            Line::Tree(row) => explorer::one_row(ctx, line, app, row, open, ui),
            Line::Said(text) => said(ctx, line, text, Role::Faint),
            Line::Head(name, count) => {
                let mut room = line.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
                let said = match count {
                    0 => name.to_string(),
                    n => format!("{name} · {n}"),
                };
                cap(ctx, &mut room, &said, Role::Ghost);
            }
            Line::Open(file) => opened(ctx, line, app, ui, file),
        },
    )
}

fn said(ctx: &mut Ctx, line: Rect, text: &str, role: Role) {
    let room = line.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
    Label::new(text, ctx.styles.small(role)).draw(ctx, room);
}

/// An open file: held while shown, italic as the preview, its dot or its cross at the right.
fn opened(ctx: &mut Ctx, line: Rect, app: &AppState, ui: &Ui, file: &Opened) {
    let path = file.path.as_str();
    let name = crate::views::name_of(path);
    if ui.losing() == Some(&Losing::Tab(file.path.clone())) {
        let question = format!("close {name} and lose its edits?");
        return super::super::asking(ctx, line, &question, "discard");
    }
    if app.workspace.active().is_some_and(|one| one.path == path) {
        glass_chosen(ctx, line);
    }
    let target = Target::OpenTab(file.path.clone());
    let on = glass_hoverable(ctx, line, target);
    let close = Target::CloseTab(file.path.clone());
    let dirty = file.new.dirty();
    let mut room = line.pad(Edges::across(ctx.tokens.md, ctx.tokens.sm));
    if dirty || on || ctx.hovered(&close) {
        let mark = match dirty && !ctx.hovered(&close) {
            true => Mark::Modified,
            false => Mark::Close,
        };
        Button::icon(mark, 0, close, Role::Faint).right(ctx, &mut room, ctx.tokens.sm);
    }
    let buffers = app.workspace.buffers();
    let preview = buffers.is_some_and(|all| all.previews(path));
    let plain = ctx.styles.body(Role::Text);
    let style = ctx.styles.emphasised(plain, false, preview);
    Label::new(name, style).draw(ctx, room);
}
