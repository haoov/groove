//! The sidebar's notes: the session's own, and the threads still open on its lines.

use groove_controllers::AppState;
use groove_gfx::Rect;
use groove_types::Note;

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::{Scroller, Target};
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::hoverable;
use groove_ui_kit::shape::square;
use groove_ui_kit::text::Label;
use groove_ui_kit::text::row;

use groove_gfx::Edges;
use groove_ui_kit::layout::{Spec, column_in, row_in};

pub(super) fn draw(ctx: &mut Ctx, body: Rect, app: &AppState, ui: &Ui) {
    let notes: Vec<(usize, &Note)> = app
        .delivery
        .shown
        .iter()
        .enumerate()
        .filter(|(_, note)| listed(note))
        .collect();
    if notes.is_empty() {
        let style = ctx.styles.small(Role::Faint);
        let [line, _] = column_in(body, [Spec::default().height(ctx.tokens.row), Spec::fill()]);
        return row(ctx, line, ctx.tokens.md, "no notes on this session", style);
    }
    let height = ctx.tokens.row;
    let at = (Scroller::Files, ui.offset(Scroller::Files));
    crate::offsets::listed(
        ctx,
        body,
        at,
        &notes,
        |_| height,
        |ctx, line, (at, note)| one(ctx, line, note, *at),
    );
}

/// A note of this session's own, or a thread still open on a line of the change.
pub(super) fn listed(note: &Note) -> bool {
    note.is_local() || (!note.resolved && note.anchor.is_some())
}

/// One note: where it stands, then what it says.
fn one(ctx: &mut Ctx, line: Rect, note: &Note, at: usize) {
    hoverable(ctx, line, Target::NoteAt(at));
    let (role, said) = match note.resolved {
        true => (Role::Faint, ctx.styles.body(Role::Faint)),
        false => (Role::Muted, ctx.styles.body(Role::Text)),
    };
    let (sm, size) = (ctx.tokens.sm, ctx.tokens.small);
    let mut room = line.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
    let wide = |width: f32| Spec::default().width(width);
    let [box_, _, rest] = row_in(room, [wide(size), wide(sm), Spec::fill()]);
    room = rest;
    let box_ = square(box_, size);
    groove_ui_kit::widgets::icon(ctx, box_, mark(note), role);
    let place = where_of(note);
    Label::new(&place, ctx.styles.small(role)).left(ctx, &mut room, sm);
    Label::new(&body_of(note), said).draw(ctx, room);
}

/// The mark a note carries: its own, or the forge's for a thread.
fn mark(note: &Note) -> Mark {
    match note.is_local() {
        true => Mark::Note,
        false => Mark::Review,
    }
}

/// The file and line a note stands on, the name alone.
fn where_of(note: &Note) -> String {
    let Some(anchor) = note.anchor.as_ref() else {
        return "on the mr".to_string();
    };
    let name = crate::views::name_of(&anchor.path);
    format!("{name} {}", anchor.start_line + 1)
}

/// What the note opens with, on one row.
fn body_of(note: &Note) -> String {
    let Some(said) = note.opening() else {
        return String::new();
    };
    let plain = groove_ui_kit::markdown::plain(&said.body);
    let first = plain.lines().next().unwrap_or_default();
    match note.replies() {
        0 => first.to_string(),
        n => format!("{first} +{n}"),
    }
}
