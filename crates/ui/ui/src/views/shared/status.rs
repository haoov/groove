use groove_controllers::AppState;
use groove_gfx::Rect;

use crate::ctx::Ctx;
use crate::style::Role;
use crate::tokens::SPINNER_MS;
use crate::widget::{Row, list};

/// One of four quarter marks, a step every `SPINNER_MS`.
const MARKS: [char; 4] = ['◐', '◓', '◑', '◒'];

/// Above `bottom`: the jobs in flight, each with a moving mark, then the last error.
pub fn draw(ctx: &mut Ctx, app: &AppState, bottom: f32) {
    let jobs: Vec<String> = app
        .pending
        .iter()
        .map(|job| format!("{} {}", spinner(ctx.tick), job.label))
        .collect();
    let (working, bad, pad) = (
        ctx.styles.small(Role::Working),
        ctx.styles.small(Role::Bad),
        ctx.tokens.md,
    );
    let mut rows: Vec<Row<'_>> = jobs.iter().map(|job| Row::new(pad, job, working)).collect();
    if let Some(error) = app.errors.last() {
        rows.push(Row::new(pad, &error.message, bad));
    }
    let height = ctx.tokens.row * rows.len() as f32;
    let width = ctx.layout.rail.w - ctx.tokens.hairline;
    let rect = Rect::new(0.0, bottom - ctx.tokens.row - height, width, height);
    list(ctx, rect, &rows, None);
}

fn spinner(tick: u64) -> char {
    MARKS[(tick / SPINNER_MS % MARKS.len() as u64) as usize]
}
