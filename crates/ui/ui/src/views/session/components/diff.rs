//! The diff tab. The surface itself comes with the alignment; for now the tab says
//! what the sidebar holds.

use groove_controllers::AppState;
use groove_gfx::Rect;

use crate::ctx::Ctx;
use crate::style::Role;
use crate::widget::row;

pub fn draw(ctx: &mut Ctx, app: &AppState, area: Rect) {
    let style = ctx.styles.body(Role::Faint);
    let files = super::files::changed(app).len();
    let text = match files {
        0 => "No change in this worktree.".to_string(),
        1 => "1 file changed. Pick it in the sidebar.".to_string(),
        n => format!("{n} files changed. Pick one in the sidebar."),
    };
    let line = Rect::new(area.x, area.y, area.w, ctx.tokens.row);
    row(ctx, line, ctx.tokens.md, &text, style);
}
