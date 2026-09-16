//! The diff tab: the open file as rows, its two gutters and its colours.

mod header;
mod row;
mod scroll;
mod surface;

use groove_controllers::AppState;
use groove_gfx::Rect;

use crate::Ui;
use crate::ctx::Ctx;
use crate::style::Role;
use crate::widget::row;

pub(crate) use scroll::{moved, scrolled};

pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui, area: Rect) {
    let Some(file) = app.workspace.opened.as_ref() else {
        return hint(ctx, app, area);
    };
    if file.long {
        let style = ctx.styles.body(Role::Faint);
        let line = Rect::new(area.x, area.y, area.w, ctx.tokens.row);
        return row(ctx, line, ctx.tokens.md, "Too long to show.", style);
    }
    let head = Rect::new(area.x, area.y, area.w, ctx.tokens.row);
    header::draw(ctx, head, app, ui, &file.path);
    let body = Rect::new(area.x, head.bottom(), area.w, area.h - head.h);
    surface::rows(ctx, body, file, ui);
}

/// What the tab says with nothing open.
fn hint(ctx: &mut Ctx, app: &AppState, area: Rect) {
    let style = ctx.styles.body(Role::Faint);
    let files = super::files::changed(app).len();
    let text = match files {
        0 => "No change in this worktree.".to_string(),
        1 => "1 file changed. Open it in the sidebar.".to_string(),
        n => format!("{n} files changed. Open one in the sidebar."),
    };
    let line = Rect::new(area.x, area.y, area.w, ctx.tokens.row);
    row(ctx, line, ctx.tokens.md, &text, style);
}
