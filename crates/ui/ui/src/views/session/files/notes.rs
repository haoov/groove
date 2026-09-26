//! The sidebar's notes: the session's own, and the threads still open on its lines.

use groove_controllers::AppState;
use groove_gfx::Rect;
use groove_types::Note;

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::{Scroller, Target};
use crate::mark::Mark;
use crate::style::Role;
use crate::widget::{elide, row};

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
    let extent = (height * notes.len() as f32 - body.h).max(0.0);
    ctx.scrolls(Scroller::Files, extent);
    let scroll = ui.session.files.min(extent);
    ctx.clipped(body, |ctx| {
        let mut y = body.y - scroll;
        for (at, note) in notes {
            let line = Rect::new(body.x, y, body.w, height);
            if y + height >= body.y && y <= body.bottom() {
                one(ctx, line, note, at, ui);
            }
            y += height;
        }
    });
}

/// A note of this session's own, or a thread still open on a line of the change.
pub(super) fn listed(note: &Note) -> bool {
    note.is_local() || (!note.resolved && note.anchor.is_some())
}

/// One note: where it stands, then what it says.
fn one(ctx: &mut Ctx, line: Rect, note: &Note, at: usize, ui: &Ui) {
    let target = Target::NoteAt(at);
    if ui.hover.as_ref() == Some(&target) {
        ctx.quad(line, ctx.styles.hover());
    }
    ctx.hit(line, target);
    let role = match note.resolved {
        true => Role::Faint,
        false => Role::Muted,
    };
    let size = ctx.tokens.small;
    let box_ = crate::widget::box_in(line, line.x + ctx.tokens.md, size);
    ctx.icon(box_, mark(note), 0, ctx.styles.color(role));
    let at = ctx.tokens.md + size + ctx.tokens.sm;
    let place = ctx.styles.small(role);
    let text = where_of(note);
    let width = ctx.measure(&text, &place);
    row(ctx, line, at, &text, place);
    let said = match note.resolved {
        true => ctx.styles.body(Role::Faint),
        false => ctx.styles.body(Role::Text),
    };
    let start = at + width + ctx.tokens.sm;
    let room = (line.w - start - ctx.tokens.md).max(0.0);
    let words = elide(ctx, &body_of(note), &said, room);
    row(ctx, line, start, &words, said);
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
