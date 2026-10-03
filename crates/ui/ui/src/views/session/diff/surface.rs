//! Where the rows are drawn: one column, or the two sides beside each other.

use groove_gfx::Rect;
use groove_types::{DiffView, RowKind};

use groove_controllers::AppState;

use super::notes::{Inline, Slot};
use super::row::{Drawn, Side, count, drawn};
use super::words::{Words, words_of};
use crate::Ui;
use crate::components::{
    Acting, Gutters, Line, Noted, Rows, across_extent, chars_of, code, height, visible,
};
use crate::ctx::Ctx;
use crate::hit::{Scroller, Target};
use crate::views::session::Face;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::base::tokens::{NOTE_BY, NOTE_SLACK};

pub(super) fn rows(ctx: &mut Ctx, body: Rect, app: &AppState, ui: &Ui) {
    match ui.session.face() {
        Face::Stream(DiffView::Split) => beside(ctx, body, app, ui),
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
    groove_ui_kit::shape::side_rule(ctx, body, left.right(), rule);
    let view = Face::Stream(DiffView::Split);
    surface(ctx, left, app, ui, view, Side::Old, false);
    surface(ctx, right, app, ui, view, Side::New, true);
}

/// Draws the rows the surface has room for, and says where a click can land.
fn surface(
    ctx: &mut Ctx,
    rect: Rect,
    app: &AppState,
    ui: &Ui,
    view: Face,
    side: Side,
    clickable: bool,
) {
    let inline = Inline::of(app, ui, view);
    let total = inline.total(count(app, view));
    let scroll = scroll_of(ctx, ui, rect, total);
    let window = visible(ctx, rect, total, scroll);
    let slots: Vec<Slot> = window.clone().map(|row| inline.slot(row)).collect();
    let code_rows = inline.code_window(window.clone());
    let rows = drawn(app, ui, view, side, code_rows.clone());
    let gutters = gutters_of(&rows);
    let words: Vec<Words> = slots.iter().map(|slot| words_of(app, ui, *slot)).collect();
    let said = super::blame::shown(ctx, app, ui, side != Side::Old);
    let held = Held {
        slots: &slots,
        rows: &rows,
        gutters: &gutters,
        words: &words,
        noted: &inline.noted(&slots),
        first: code_rows.start,
        notes: side != Side::Old,
        said: said.as_ref(),
    };
    let lines = lines_of(ctx, app, held);
    let numbers = numbers(app, view);
    let across = across_of(ctx, ui, numbers, rect, &rows);
    if clickable {
        asks(ctx, rect, code_rows.clone(), numbers, (scroll, across));
    }
    let shown = Rows {
        lines: &lines,
        first: window.start,
        gutters: numbers,
        across,
    };
    let drawn = code(ctx, rect, shown, scroll);
    if clickable {
        let at = (numbers, view, side);
        super::offers::marks(ctx, &drawn, &slots, &rows, code_rows.start, at);
    }
}

/// How far the surface is scrolled down, inside what its `total` rows allow.
fn scroll_of(ctx: &mut Ctx, ui: &Ui, rect: Rect, total: usize) -> f32 {
    let extent = (height(ctx, total) - rect.h).max(0.0);
    ctx.app.hits.scrolls(Scroller::Code, extent);
    ui.session.scroll().min(extent)
}

/// How far the rows in view are scrolled sideways, inside what their widest allows.
fn across_of(ctx: &mut Ctx, ui: &Ui, numbers: Gutters, rect: Rect, rows: &[Drawn]) -> f32 {
    let reach = across_extent(ctx, numbers, rect, rows.iter().map(|row| row.text.as_str()));
    ctx.app.hits.scrolls(Scroller::Across, reach);
    ui.session.across().min(reach)
}

/// What every row of the view says in its number columns.
fn gutters_of(rows: &[Drawn]) -> Vec<Vec<&str>> {
    rows.iter()
        .map(|row| row.gutters.iter().map(String::as_str).collect())
        .collect()
}

/// The rows in view, and where a caret can land in them.
fn asks(
    ctx: &mut Ctx,
    rect: Rect,
    rows: std::ops::Range<usize>,
    numbers: Gutters,
    (scroll, across): (f32, f32),
) {
    ctx.app.hits.showing(rows);
    ctx.hit(rect, Target::Code);
    let chars = chars_of(ctx, numbers, rect, (scroll, across));
    let cols = note_cols(ctx, rect, chars.left + across);
    ctx.app.hits.characters(chars);
    ctx.app.hits.wraps(cols);
}

/// How many characters of a note fit between its column and who said it.
fn note_cols(ctx: &mut Ctx, rect: Rect, left: f32) -> usize {
    let sample = "the quick brown fox jumps over the lazy dog";
    let words = ctx.styles.body(Role::Text);
    let each = ctx.measure(sample, &words) / sample.len() as f32 * NOTE_SLACK;
    let room = rect.right() - left - ctx.tokens.md - each * NOTE_BY;
    (room / each).floor().max(0.0) as usize
}

/// What the surface holds for the rows it is about to draw.
struct Held<'a> {
    slots: &'a [Slot],
    rows: &'a [Drawn],
    gutters: &'a [Vec<&'a str>],
    words: &'a [Words],
    /// Which rows of the window a note stands on.
    noted: &'a [bool],
    /// The row of the view the first of `rows` is.
    first: usize,
    /// Whether this side draws the notes; the old side of a split keeps their rows blank.
    notes: bool,
    /// The blame of the line the caret rests on.
    said: Option<&'a super::blame::Said>,
}

/// Every row of the window as the code widget takes it.
fn lines_of<'a>(ctx: &mut Ctx, app: &AppState, held: Held<'a>) -> Vec<Line<'a>> {
    held.slots
        .iter()
        .enumerate()
        .map(|(on, slot)| match slot {
            Slot::Note { .. } | Slot::Acts { .. } | Slot::Typed { .. } if !held.notes => {
                Line::new("")
            }
            Slot::Code(at) => match held.rows.get(at - held.first) {
                Some(row) => lined(ctx, row, &held.gutters[at - held.first])
                    .noted(held.noted[on])
                    .blame(super::blame::on(row, held.said)),
                None => Line::new(""),
            },
            Slot::Note { at, row } => Line::note(
                &held.words[on].body,
                Noted {
                    author: &held.words[on].author,
                    lines: &held.words[on].lines,
                    opens: *row == 0,
                    resolved: app.delivery.shown.get(*at).is_some_and(|one| one.resolved),
                    prose: held.words[on].prose.as_ref(),
                },
            ),
            Slot::Acts { at } => match app.delivery.shown.get(*at) {
                Some(note) => Line::acting(acting_of(app, note)),
                None => Line::new(""),
            },
            Slot::Typed { row } => Line::note(
                &held.words[on].body,
                Noted {
                    author: &held.words[on].author,
                    lines: &held.words[on].lines,
                    opens: *row == 0,
                    resolved: false,
                    prose: None,
                },
            ),
        })
        .collect()
}

/// What a note's own row of buttons acts on.
fn acting_of(app: &AppState, note: &groove_types::Note) -> Acting {
    Acting {
        origin: note.origin.clone(),
        resolved: note.resolved,
        thread: !note.is_local(),
        post: app
            .session
            .selected_worktree()
            .is_some_and(|worktree| app.delivery.has_mr(&worktree.id)),
    }
}

/// One row as the code widget takes it: a head, a gap, or a line of text.
fn lined<'a>(ctx: &mut Ctx, row: &'a Drawn, gutters: &'a [&'a str]) -> Line<'a> {
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

/// How wide the line numbers stand: one column a side in split and file, two in inline.
pub(super) fn numbers(app: &AppState, view: Face) -> Gutters {
    let digits = match view {
        Face::File => match app.workspace.active() {
            Some(file) => file.new.lines().to_string().len(),
            None => 1,
        },
        _ => app.workspace.changes.digits(),
    };
    Gutters {
        cells: match view {
            Face::Stream(DiffView::Inline) => 2,
            _ => 1,
        },
        digits,
    }
}
