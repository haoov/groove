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
        let line = Rect::new(body.x, body.y, body.w, ctx.tokens.row);
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
    let box_ = square(room.take_left(size), size);
    room.take_left(sm);
    ctx.icon(box_, mark(note), 0, ctx.styles.color(role));
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
    let name = anchor.path.rsplit('/').next().unwrap_or(&anchor.path);
    format!("{name} {}", anchor.start_line + 1)
}

/// What the note opens with, on one row.
fn body_of(note: &Note) -> String {
    let Some(said) = note.opening() else {
        return String::new();
    };
    let first = said.body.lines().next().unwrap_or_default().trim();
    match note.replies() {
        0 => first.to_string(),
        n => format!("{first} +{n}"),
    }
}
