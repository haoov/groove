use groove_controllers::AppState;
use groove_gfx::Rect;

use crate::ctx::Ctx;
use crate::mark::Mark;
use crate::style::Role;
use crate::widget::{Row, list, turn};

/// Above `bottom`: the jobs in flight, each with a moving mark, then the last error.
pub fn draw(ctx: &mut Ctx, app: &AppState, bottom: f32) {
    let turn = turn(ctx.tick);
    let (working, bad, pad) = (
        ctx.styles.small(Role::Working),
        ctx.styles.small(Role::Bad),
        ctx.tokens.md,
    );
    let mut rows: Vec<Row<'_>> = app
        .pending
        .iter()
        .map(|job| Row::new(pad, &job.label, working).turning(Mark::Busy, turn))
        .collect();
    if let Some(error) = app.errors.last() {
        rows.push(Row::new(pad, &error.message, bad));
    }
    let height = ctx.tokens.row * rows.len() as f32;
    let width = ctx.layout.rail.w - ctx.tokens.hairline;
    let rect = Rect::new(0.0, bottom - ctx.tokens.row - height, width, height);
    list(ctx, rect, &rows, None);
}
