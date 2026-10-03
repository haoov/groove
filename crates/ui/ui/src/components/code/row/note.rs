//! A note's own rows: what it says, and what it offers.

use groove_gfx::Rect;

use super::super::{Acting, Noted};
use crate::ctx::Ctx;
use crate::hit::{NoteButton, Target};
use groove_ui_kit::base::ground::Ground;
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::box_in;
use groove_ui_kit::text::{elide, row};
use groove_ui_kit::widgets::Button;
use groove_ui_kit::widgets::code::Block;

/// One row of a note: its ground, its mark, its author, its words.
pub(super) fn note(ctx: &mut Ctx, line: Rect, text: &str, said: Noted<'_>, gutter: Block) {
    groove_ui_kit::shape::ground(ctx, line, Ground::Deep);
    let role = match said.resolved {
        true => Role::Faint,
        false => Role::Text,
    };
    if said.opens {
        gutter_mark(ctx, line, gutter, Role::Muted);
    }
    let by = by(ctx, line, said);
    let at = gutter.content(ctx, line);
    let words = ctx.styles.body(role);
    let rect = Rect::new(at, line.y, (by - at - ctx.tokens.sm).max(0.0), line.h);
    if let Some(prose) = said.prose {
        let link = |url: &str| Target::Link(url.to_string());
        return groove_ui_kit::markdown::row(ctx, rect, prose, role, &link);
    }
    let text = elide(ctx, text, &words, rect.w);
    row(ctx, rect, 0.0, &text, words);
}

/// Who said it and which lines, at the row's end; returns where they start.
fn by(ctx: &mut Ctx, line: Rect, said: Noted<'_>) -> f32 {
    let style = ctx.styles.small(Role::Faint);
    let mut at = line.right() - ctx.tokens.md;
    for text in [said.author, said.lines] {
        if text.is_empty() {
            continue;
        }
        let width = ctx.measure(text, &style);
        at -= width;
        row(ctx, Rect::new(at, line.y, width, line.h), 0.0, text, style);
        at -= ctx.tokens.sm;
    }
    at
}

/// The note's mark, right-aligned where the line's own number would stand.
fn gutter_mark(ctx: &mut Ctx, line: Rect, gutter: Block, role: Role) {
    let size = ctx.tokens.small;
    let at = gutter.numbers_end(ctx, line) - size;
    groove_ui_kit::widgets::icon(ctx, box_in(line, at, size), Mark::Note, role);
}

/// What a note offers, as buttons from the column its words start on.
pub(super) fn acts(ctx: &mut Ctx, line: Rect, acting: &Acting, gutter: Block) {
    groove_ui_kit::shape::ground(ctx, line, Ground::Deep);
    let offered = offered(acting);
    let mut at = gutter.content(ctx, line);
    for button in offered {
        at = acted(ctx, line, at, *button, acting) + ctx.tokens.sm;
    }
}

/// What this note lets a hand do: a thread is answered, its own is written.
fn offered(acting: &Acting) -> &'static [NoteButton] {
    match (acting.thread, acting.post) {
        (true, _) => &[NoteButton::Reply, NoteButton::Resolve],
        (false, true) => &[
            NoteButton::Edit,
            NoteButton::Resolve,
            NoteButton::Delete,
            NoteButton::Post,
        ],
        (false, false) => &[NoteButton::Edit, NoteButton::Resolve, NoteButton::Delete],
    }
}

/// One button of a note's row, from `at`. Returns where it ends.
fn acted(ctx: &mut Ctx, line: Rect, at: f32, button: NoteButton, acting: &Acting) -> f32 {
    let label = button.label(acting.resolved);
    let target = Target::Note(acting.origin.clone(), button);
    let (raised, action) = (ctx.styles.raised(), ctx.styles.action());
    let pressed = Button::new(label, target, Role::Muted, raised).hover(action);
    pressed.at(ctx, line, at).right()
}
