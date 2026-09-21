//! The diff tab: the open file as rows, its two gutters and its colours.

mod finder;
mod header;
mod map;
mod pinned;
mod row;
mod scroll;
mod surface;

use groove_controllers::AppState;
use groove_controllers::workspace_service::At;
use groove_gfx::Rect;
use groove_types::DiffView;

use crate::Ui;
use crate::ctx::Ctx;
use crate::style::Role;
use crate::widget::{first, row};

pub(crate) use map::total as rows_of;
pub(crate) use row::{line_at, text_at};
pub(crate) use scroll::scrolled;

pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui, area: Rect) {
    if app.workspace.changes.is_empty() && !as_a_file(app, ui) {
        return hint(ctx, app, area);
    }
    let head = Rect::new(area.x, area.y, area.w, ctx.tokens.row);
    header::draw(ctx, head, app, ui, &standing(ctx, app, ui));
    let column = ctx.tokens.map;
    let body = Rect::new(area.x, head.bottom(), area.w - column, area.h - head.h);
    map::draw(
        ctx,
        Rect::new(body.right(), body.y, column, body.h),
        body,
        app,
        ui,
    );
    match whole_file(app, ui) {
        Some(true) => said(ctx, body, "Too long to show."),
        Some(false) => said(ctx, body, "Open a file in the sidebar."),
        None => {
            surface::rows(ctx, body, app, ui);
            pinned::draw(ctx, body, app, ui, surface::numbers(app, ui.session.view));
            finder::draw(ctx, body, ui);
        }
    }
}

/// Whether the file view has a file to draw, whatever the change holds.
fn as_a_file(app: &AppState, ui: &Ui) -> bool {
    ui.session.view == DiffView::Editor && app.workspace.opened.is_some()
}

/// What the file view has to say instead of rows: nothing open, or too long to show.
fn whole_file(app: &AppState, ui: &Ui) -> Option<bool> {
    if ui.session.view != DiffView::Editor {
        return None;
    }
    match app.workspace.opened.as_ref() {
        Some(file) => file.long.then_some(true),
        None => Some(false),
    }
}

/// The file the header names: the one being edited, else the one under the top row.
fn standing(ctx: &Ctx, app: &AppState, ui: &Ui) -> String {
    if let Some(open) = app.workspace.opened.as_ref() {
        return open.path.clone();
    }
    if ui.session.view == DiffView::Editor {
        return String::new();
    }
    let top = first(ctx.tokens.line, ui.session.diff);
    match app.workspace.changes.at(top) {
        Some(At::Band(file) | At::Head(file) | At::Row(file, _)) => file.path.clone(),
        None => String::new(),
    }
}

fn said(ctx: &mut Ctx, area: Rect, text: &str) {
    let style = ctx.styles.body(Role::Faint);
    let line = Rect::new(area.x, area.y, area.w, ctx.tokens.row);
    row(ctx, line, ctx.tokens.md, text, style);
}

/// What the tab says with nothing to show.
fn hint(ctx: &mut Ctx, app: &AppState, area: Rect) {
    let files = super::files::changed(app).len();
    let text = match files {
        0 => "No change in this worktree.".to_string(),
        _ => "Reading the change\u{2026}".to_string(),
    };
    said(ctx, area, &text);
}
