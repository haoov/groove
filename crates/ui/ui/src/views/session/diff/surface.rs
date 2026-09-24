//! Where the rows are drawn: one column, or the two sides beside each other.

use groove_gfx::Rect;
use groove_types::{DiffView, RowKind};

use groove_controllers::AppState;

use super::notes::{Inline, Slot, lines, said};
use super::row::{Drawn, Side, count, drawn};
use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::{Scroller, Target};
use crate::style::Role;
use crate::tokens::{NOTE_BY, NOTE_SLACK};
use crate::widget::{Acting, Gutters, Line, Noted, Rows, chars_of, code, height, visible};

/// Who a note left in the app is by.
pub(crate) const AUTHOR: &str = "you";

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
    let inline = Inline::of(app, ui, view);
    let total = inline.total(count(app, view));
    let extent = (height(ctx, total) - rect.h).max(0.0);
    ctx.scrolls(Scroller::Code, extent);
    let scroll = ui.session.diff.min(extent);
    let window = visible(ctx, rect, total, scroll);
    let slots: Vec<Slot> = window.clone().map(|row| inline.slot(row)).collect();
    let code_rows = inline.code_window(window.clone());
    let rows = drawn(app, ui, view, side, code_rows.clone());
    let gutters = gutters_of(&rows);
    let words: Vec<Words> = slots.iter().map(|slot| words_of(app, ui, *slot)).collect();
    let noted: Vec<bool> = slots
        .iter()
        .map(|slot| matches!(slot, Slot::Code(at) if inline.notes(*at)))
        .collect();
    let held = Held {
        slots: &slots,
        rows: &rows,
        gutters: &gutters,
        words: &words,
        noted: &noted,
        hovered: hovered(ui),
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
        let at = (numbers, view, side);
        super::offers::marks(ctx, &drawn, &slots, &rows, code_rows.start, at);
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
    let cols = note_cols(ctx, rect, chars.left);
    ctx.characters(chars);
    ctx.wraps(cols);
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
    /// The note button the pointer stands on, and whose note it is.
    hovered: Option<(groove_types::NoteOrigin, crate::hit::NoteButton)>,
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
                Some(row) => lined(ctx, row, &held.gutters[at - held.first]).noted(held.noted[on]),
                None => Line::new(""),
            },
            Slot::Note { at, row } => Line::note(
                &held.words[on].body,
                Noted {
                    author: &held.words[on].author,
                    lines: &held.words[on].lines,
                    opens: *row == 0,
                    resolved: app.workspace.notes.get(*at).is_some_and(|one| one.resolved),
                },
            ),
            Slot::Acts { at } => match app.workspace.notes.get(*at) {
                Some(note) => {
                    let acting = acting_of(app, note);
                    Line::acting(Acting {
                        hovered: on_it(held.hovered.as_ref(), &acting.origin),
                        ..acting
                    })
                }
                None => Line::new(""),
            },
            Slot::Typed => Line::note(
                &held.words[on].body,
                Noted {
                    author: &held.words[on].author,
                    lines: &held.words[on].lines,
                    opens: true,
                    resolved: false,
                },
            ),
        })
        .collect()
}

/// The note button under the pointer, and whose note it belongs to.
fn hovered(ui: &Ui) -> Option<(groove_types::NoteOrigin, crate::hit::NoteButton)> {
    match &ui.hover {
        Some(crate::hit::Target::Note(id, button)) => Some((id.clone(), *button)),
        _ => None,
    }
}

/// The button of this note the pointer stands on.
fn on_it(
    hovered: Option<&(groove_types::NoteOrigin, crate::hit::NoteButton)>,
    origin: &groove_types::NoteOrigin,
) -> Option<crate::hit::NoteButton> {
    hovered
        .filter(|(whose, _)| whose == origin)
        .map(|(_, button)| *button)
}

/// What a note's own row of buttons acts on.
fn acting_of(app: &AppState, note: &groove_types::Note) -> Acting {
    Acting {
        origin: note.origin.clone(),
        resolved: note.resolved,
        thread: !note.is_local(),
        post: app.workspace.delivery.mr.is_some(),
        hovered: None,
    }
}

/// What a note row says: the words, who said them, and the lines they are about.
#[derive(Default)]
struct Words {
    author: String,
    lines: String,
    body: String,
}

/// Who said what on a note row; a row of code says nothing.
fn words_of(app: &AppState, ui: &Ui, slot: Slot) -> Words {
    match slot {
        Slot::Note { at, row } => match app.workspace.notes.get(at) {
            Some(note) => {
                let (author, body) = said(note, row, super::wrap::cols_of(ui));
                let shown = match row {
                    0 => note.anchor.as_ref().map(lines).unwrap_or_default(),
                    _ => String::new(),
                };
                Words {
                    author,
                    lines: shown,
                    body,
                }
            }
            None => Words::default(),
        },
        Slot::Typed => match ui.session.noting.as_ref() {
            Some(noting) => Words {
                author: AUTHOR.to_string(),
                lines: lines(&noting.anchor),
                body: noting.field.shown(),
            },
            None => Words::default(),
        },
        Slot::Acts { .. } | Slot::Code(_) => Words::default(),
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
