//! The column beside the rows: the whole change as bands, under a lens that drags.

use groove_controllers::AppState;
use groove_controllers::workspace_service::Aligned;
use groove_gfx::{Edges, Rect};
use groove_types::{LineMark, RowKind};

use super::notes::Inline;
use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use crate::views::session::Face;
use groove_ui_kit::base::ground::Ground;
use groove_ui_kit::shape::Panel;

pub(super) fn draw(
    ctx: &mut Ctx,
    (rect, body): (Rect, Rect),
    app: &AppState,
    ui: &Ui,
    inline: &Inline,
) {
    let total = inline.total(super::row::count(app, ui, ui.session.face()));
    if total == 0 {
        return;
    }
    groove_ui_kit::shape::ground(ctx, rect, Ground::Band);
    let per = rect.h / total as f32;
    match ui.session.face() {
        Face::File => whole(ctx, rect, per, app, inline),
        _ => change(ctx, rect, per, app, inline),
    }
    lens(ctx, rect, per, body, ui);
    ctx.hit(rect, Target::Map);
}

/// What the column stands for: the rows of the view, and the notes in them wrapped to `cols`.
pub(crate) fn total(app: &AppState, ui: &Ui, cols: usize) -> usize {
    let view = ui.session.face();
    Inline::of(app, ui, view, cols).total(super::row::count(app, ui, view))
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
    let high = (shown as f32 * per).max(ctx.tokens.hairline);
    let read = groove_ui_kit::shape::band(rect, start as f32 * per, high);
    groove_ui_kit::shape::ground(ctx, read, Ground::Hover);
}

/// The open file alone: what the change did to each of its lines.
fn whole(ctx: &mut Ctx, rect: Rect, per: f32, app: &AppState, inline: &Inline) {
    let Some(open) = app.workspace.active() else {
        return;
    };
    let mut runs: Vec<(usize, usize, LineMark)> = Vec::new();
    for (line, mark) in &open.hunked.marks {
        joined(&mut runs, (*line as usize, 1, *mark));
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
    let height = (high as f32 * per).max(ctx.tokens.hairline);
    let shown = groove_ui_kit::shape::band(rect, top as f32 * per, height);
    groove_ui_kit::shape::ground(ctx, shown, Ground::Work);
    if high == 0 {
        return;
    }
    for (at, run, mark) in runs(file) {
        marked(ctx, rect, per, inline.shifted(start + 1 + at), run, mark);
    }
}

/// One run of lines the change touched, inset from the column's edges.
fn marked(ctx: &mut Ctx, rect: Rect, per: f32, at: usize, run: usize, mark: LineMark) {
    let inset = ctx.tokens.xs;
    let high = (run as f32 * per).max(ctx.tokens.hairline);
    let run =
        groove_ui_kit::shape::band(rect, at as f32 * per, high).pad(Edges::across(inset, inset));
    groove_ui_kit::shape::ground(ctx, run, Ground::Mark(mark));
}

/// Every run of rows the change touched: where it starts, how long, and what it did.
fn runs(file: &Aligned) -> Vec<(usize, usize, LineMark)> {
    let mut runs: Vec<(usize, usize, LineMark)> = Vec::new();
    for (at, rows, kind) in file.hunked.layout.spans() {
        let mark = match kind {
            RowKind::Added => LineMark::Added,
            RowKind::Removed => LineMark::Removed,
            _ => continue,
        };
        joined(&mut runs, (at, rows, mark));
    }
    runs
}

/// A run onto the last one when it carries on from it with the same mark, else after it.
fn joined(runs: &mut Vec<(usize, usize, LineMark)>, (at, len, mark): (usize, usize, LineMark)) {
    match runs.last_mut() {
        Some(last) if last.0 + last.1 == at && last.2 == mark => last.1 += len,
        _ => runs.push((at, len, mark)),
    }
}

/// What the surface shows, outlined over the column.
fn lens(ctx: &mut Ctx, rect: Rect, per: f32, body: Rect, ui: &Ui) {
    let line = ctx.tokens.code_line;
    let high = (body.h / line * per).min(rect.h).max(ctx.tokens.xs);
    let top = ui.session.scroll() / line * per;
    let held = groove_ui_kit::shape::band(rect, top.min(rect.h - high), high);
    Panel::default().border(ctx.styles.lens()).draw(ctx, held);
}
