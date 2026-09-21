//! The column beside the rows: the whole change as bands, under a lens that drags.

use groove_controllers::AppState;
use groove_controllers::workspace_service::Aligned;
use groove_gfx::Rect;
use groove_types::{DiffView, LineMark, RowKind};

use super::notes::Inline;
use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;

pub(super) fn draw(ctx: &mut Ctx, rect: Rect, body: Rect, app: &AppState, ui: &Ui) {
    let total = total(app, ui);
    if total == 0 {
        return;
    }
    ctx.quad(rect, ctx.styles.band());
    let per = rect.h / total as f32;
    let inline = Inline::of(app, ui.session.view);
    match ui.session.view {
        DiffView::Editor => whole(ctx, rect, per, app, &inline),
        _ => change(ctx, rect, per, app, &inline),
    }
    lens(ctx, rect, per, body, ui);
    ctx.hit(rect, Target::Map);
}

/// What the column stands for: the rows of the view, and the notes in them.
pub(crate) fn total(app: &AppState, ui: &Ui) -> usize {
    let view = ui.session.view;
    Inline::of(app, view).total(super::row::count(app, view))
}

/// Every changed file, one band under another.
fn change(ctx: &mut Ctx, rect: Rect, per: f32, app: &AppState, inline: &Inline) {
    for (start, shown, file) in app.workspace.changes.placed() {
        let run = run(inline, start, shown);
        match super::row::is_read(app, &file.path) {
            true => read(ctx, rect, per, run),
            false => band(ctx, rect, per, run, file, inline, start),
        }
    }
}

/// Where a run of rows stands and how tall it is, with the notes inside it.
fn run(inline: &Inline, start: usize, shown: usize) -> (usize, usize) {
    let top = inline.shifted(start);
    (top, inline.shifted(start + shown) - top)
}

/// A file already read: its own ground, and none of its marks.
fn read(ctx: &mut Ctx, rect: Rect, per: f32, at: (usize, usize)) {
    let (start, shown) = at;
    let top = rect.y + start as f32 * per;
    let high = (shown as f32 * per).max(ctx.tokens.hairline);
    ctx.quad(Rect::new(rect.x, top, rect.w, high), ctx.styles.hover());
}

/// The open file alone: what the change did to each of its lines.
fn whole(ctx: &mut Ctx, rect: Rect, per: f32, app: &AppState, inline: &Inline) {
    let Some(open) = app.workspace.opened.as_ref() else {
        return;
    };
    let mut runs: Vec<(usize, usize, LineMark)> = Vec::new();
    for (line, mark) in &open.marks {
        let at = *line as usize;
        match runs.last_mut() {
            Some(last) if last.0 + last.1 == at && last.2 == *mark => last.1 += 1,
            _ => runs.push((at, 1, *mark)),
        }
    }
    for (at, run, mark) in runs {
        marked(ctx, rect, per, inline.shifted(at), run, mark);
    }
}

/// One file: its own ground, and a mark for every run its change touched.
fn band(
    ctx: &mut Ctx,
    rect: Rect,
    per: f32,
    at: (usize, usize),
    file: &Aligned,
    inline: &Inline,
    start: usize,
) {
    let (top, high) = at;
    let y = rect.y + top as f32 * per;
    let height = (high as f32 * per).max(ctx.tokens.hairline);
    ctx.quad(Rect::new(rect.x, y, rect.w, height), ctx.styles.ground());
    if high == 0 {
        return;
    }
    for (at, run, mark) in runs(file) {
        marked(ctx, rect, per, inline.shifted(start + 1 + at), run, mark);
    }
}

/// One run of lines the change touched, inset from the column's edges.
fn marked(ctx: &mut Ctx, rect: Rect, per: f32, at: usize, run: usize, mark: LineMark) {
    let color = ctx.styles.mark(mark);
    let inset = ctx.tokens.xs;
    let y = rect.y + at as f32 * per;
    let high = (run as f32 * per).max(ctx.tokens.hairline);
    ctx.quad(
        Rect::new(rect.x + inset, y, rect.w - inset * 2.0, high),
        color,
    );
}

/// Every run of rows the change touched: where it starts, how long, and what it did.
fn runs(file: &Aligned) -> Vec<(usize, usize, LineMark)> {
    let mut runs: Vec<(usize, usize, LineMark)> = Vec::new();
    for (at, row) in file.rows.iter().enumerate() {
        let mark = match row.kind {
            RowKind::Added => LineMark::Added,
            RowKind::Removed => LineMark::Removed,
            _ => continue,
        };
        match runs.last_mut() {
            Some(last) if last.0 + last.1 == at && last.2 == mark => last.1 += 1,
            _ => runs.push((at, 1, mark)),
        }
    }
    runs
}

/// What the surface shows, outlined over the column.
fn lens(ctx: &mut Ctx, rect: Rect, per: f32, body: Rect, ui: &Ui) {
    let line = ctx.tokens.line;
    let high = (body.h / line * per).min(rect.h).max(ctx.tokens.xs);
    let top = rect.y + ui.session.diff / line * per;
    let held = Rect::new(rect.x, top.min(rect.bottom() - high), rect.w, high);
    ctx.border(held, ctx.styles.lens());
}
