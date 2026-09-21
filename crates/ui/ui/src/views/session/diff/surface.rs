//! Where the rows are drawn: one column, or the two sides beside each other.

use groove_gfx::Rect;
use groove_types::{DiffView, RowKind};

use groove_controllers::AppState;

use super::notes::{Inline, Slot, said};
use super::row::{Drawn, Side, count, drawn};
use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::{Scroller, Target};
use crate::widget::{Gutters, Line, Noted, Rows, chars_of, code, head_mark, height, visible};

pub(super) fn rows(ctx: &mut Ctx, body: Rect, app: &AppState, ui: &Ui) {
    match ui.session.view {
        DiffView::Split => beside(ctx, body, app, ui),
        view => surface(ctx, body, app, ui, view, Side::New, true),
    }
}

/// The old on the left, the new on the right, one alignment between them.
fn beside(ctx: &mut Ctx, body: Rect, app: &AppState, ui: &Ui) {
    let thickness = ctx.tokens.hairline;
    let half = ((body.w - thickness) / 2.0).floor();
    let left = Rect::new(body.x, body.y, half, body.h);
    let right = Rect::new(
        left.right() + thickness,
        body.y,
        body.w - half - thickness,
        body.h,
    );
    let rule = ctx.styles.line();
    ctx.quad(Rect::new(left.right(), body.y, thickness, body.h), rule);
    let view = DiffView::Split;
    surface(ctx, left, app, ui, view, Side::Old, false);
    surface(ctx, right, app, ui, view, Side::New, true);
}

/// Draws the rows the surface has room for, and says where a click can land.
fn surface(
    ctx: &mut Ctx,
    rect: Rect,
    app: &AppState,
    ui: &Ui,
    view: DiffView,
    side: Side,
    clickable: bool,
) {
    let inline = Inline::of(app, view);
    let total = inline.total(count(app, view));
    let extent = (height(ctx, total) - rect.h).max(0.0);
    ctx.scrolls(Scroller::Code, extent);
    let scroll = ui.session.diff.min(extent);
    let window = visible(ctx, rect, total, scroll);
    let slots: Vec<Slot> = window.clone().map(|row| inline.slot(row)).collect();
    let code_rows = inline.code_window(window.clone());
    let rows = drawn(app, ui, view, side, code_rows.clone());
    let gutters = gutters_of(&rows);
    let words: Vec<(String, String)> = slots.iter().map(|slot| words_of(app, *slot)).collect();
    let held = Held {
        slots: &slots,
        rows: &rows,
        gutters: &gutters,
        words: &words,
        first: code_rows.start,
    };
    let lines = lines_of(ctx, app, held);
    let numbers = numbers(app, view);
    if clickable {
        asks(ctx, rect, code_rows.clone(), numbers, scroll);
    }
    let shown = Rows {
        lines: &lines,
        first: window.start,
        gutters: numbers,
    };
    let drawn = code(ctx, rect, shown, scroll);
    if clickable {
        marks(ctx, &drawn, &slots, &rows, code_rows.start);
    }
}

/// What every row of the view says in its number columns.
fn gutters_of(rows: &[Drawn]) -> Vec<Vec<&str>> {
    rows.iter()
        .map(|row| row.gutters.iter().map(String::as_str).collect())
        .collect()
}

/// The rows in view, and where a caret can land in them.
fn asks(ctx: &mut Ctx, rect: Rect, rows: std::ops::Range<usize>, numbers: Gutters, scroll: f32) {
    ctx.showing(rows);
    ctx.hit(rect, Target::Code);
    let chars = chars_of(ctx, numbers, rect, scroll);
    ctx.characters(chars);
}

/// What the surface holds for the rows it is about to draw.
struct Held<'a> {
    slots: &'a [Slot],
    rows: &'a [Drawn],
    gutters: &'a [Vec<&'a str>],
    words: &'a [(String, String)],
    /// The row of the view the first of `rows` is.
    first: usize,
}

/// Every row of the window as the code widget takes it.
fn lines_of<'a>(ctx: &mut Ctx, app: &AppState, held: Held<'a>) -> Vec<Line<'a>> {
    held.slots
        .iter()
        .enumerate()
        .map(|(on, slot)| match slot {
            Slot::Code(at) => match held.rows.get(at - held.first) {
                Some(row) => lined(ctx, row, &held.gutters[at - held.first]),
                None => Line::new(""),
            },
            Slot::Note { at, row } => Line::note(
                &held.words[on].1,
                Noted {
                    author: &held.words[on].0,
                    opens: *row == 0,
                    resolved: app.workspace.notes.get(*at).is_some_and(|one| one.resolved),
                },
            ),
        })
        .collect()
}

/// Who said what on a note row; a row of code says nothing.
fn words_of(app: &AppState, slot: Slot) -> (String, String) {
    match slot {
        Slot::Note { at, row } => match app.workspace.notes.get(at) {
            Some(note) => said(note, row),
            None => (String::new(), String::new()),
        },
        Slot::Code(_) => (String::new(), String::new()),
    }
}

/// One row as the code widget takes it: a band, a head, a gap, or a line of text.
fn lined<'a>(ctx: &mut Ctx, row: &'a Drawn, gutters: &'a [&'a str]) -> Line<'a> {
    if row.band {
        return Line::band(&row.text);
    }
    if row.head {
        return Line::head(&row.text).folded(row.folded).read(row.read);
    }
    if let RowKind::Gap(_) = row.kind {
        return Line::banner(&row.text);
    }
    let line = Line::new(&row.text)
        .gutters(gutters)
        .spans(&row.spans)
        .words(&row.words, ctx.styles.word(row.kind, row.mark))
        .found(&row.found)
        .standing(row.standing)
        .mark(row.mark.map(|mark| ctx.styles.mark(mark)))
        .caret(row.caret)
        .held(row.held);
    match ctx.styles.row_ground(row.kind) {
        Some(color) => line.ground(color),
        None => line,
    }
}

/// What a head row offers: the row itself folds, its box marks the file read.
fn marks(ctx: &mut Ctx, drawn: &[Rect], slots: &[Slot], rows: &[Drawn], first: usize) {
    for (line, slot) in drawn.iter().zip(slots) {
        let Slot::Code(at) = slot else {
            continue;
        };
        let Some(row) = rows.get(at - first) else {
            continue;
        };
        let Some(path) = row.file.as_ref().filter(|_| row.head) else {
            continue;
        };
        ctx.hit(*line, Target::Head(path.clone()));
        ctx.hit(head_mark(ctx, *line), Target::Read(path.clone()));
    }
}

/// How wide the numbers stand: one column a side in split and file, two in inline,
/// one width over the whole change.
pub(super) fn numbers(app: &AppState, view: DiffView) -> Gutters {
    let digits = match view {
        DiffView::Editor => match app.workspace.opened.as_ref() {
            Some(file) => file.new.lines().to_string().len(),
            None => 1,
        },
        _ => app.workspace.changes.digits(),
    };
    Gutters {
        cells: match view {
            DiffView::Inline => 2,
            _ => 1,
        },
        digits,
    }
}
