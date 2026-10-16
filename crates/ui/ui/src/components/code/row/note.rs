//! A note's own rows: what it says, and what it offers.

use groove_gfx::Rect;

use super::super::{Acting, Header, Line, Noted, Room, label_role};
use crate::ctx::Ctx;
use crate::hit::{NoteButton, Target};
use groove_ui_kit::base::ground::Ground;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::{card_row, framed, top_rule};
use groove_ui_kit::text::{Label, ago, elide, row};
use groove_ui_kit::widgets::code::Block;
use groove_ui_kit::widgets::{Badge, Button, typed_line};

/// One row of a note: its part of the card, its mark, its author, its words.
pub(super) fn note(ctx: &mut Ctx, line: Rect, code: &Line<'_>, said: Noted<'_>, gutter: Block) {
    let card = slotted(ctx, line, gutter);
    card_row(ctx, card, code.edges);
    let role = match said.resolved {
        true => Role::Faint,
        false => Role::Text,
    };
    if let Some(head) = said.head {
        return header(ctx, (line, card), gutter, head, role);
    }
    let by = by(ctx, card, said);
    let at = gutter.content(ctx, line);
    let words = ctx.styles.body(role);
    if let Some(prose) = said.prose {
        let at = at + ctx.tokens.xl;
        let rect = Rect::new(at, line.y, (by - at - ctx.tokens.sm).max(0.0), line.h);
        let link = |url: &str| Target::Link(url.to_string());
        return groove_ui_kit::markdown::row(ctx, rect, prose, role, &link);
    }
    let rect = Rect::new(at, line.y, (by - at - ctx.tokens.sm).max(0.0), line.h);
    let field = Rect::new(
        at - ctx.tokens.xs,
        line.y,
        card.right() - ctx.tokens.sm - at,
        line.h,
    );
    groove_ui_kit::shape::ground(ctx, field, Ground::Deep);
    if let Some(edge) = said.field {
        let colour = match edge.lit {
            true => ctx.styles.color(Role::Accent),
            false => ctx.styles.line(),
        };
        framed(ctx, field, (edge.first, edge.last), colour);
    }
    let text = elide(ctx, code.text, &words, rect.w);
    typed_line(ctx, rect, (&text, words), (code.caret, code.held));
}

/// A comment's header: initials, author, source and label from the left; lines and age at the end.
fn header(ctx: &mut Ctx, (line, card): (Rect, Rect), gutter: Block, head: &Header, role: Role) {
    if head.reply {
        let rule = ctx.styles.line();
        top_rule(ctx, card, rule);
    }
    let at = gutter.content(ctx, line);
    let mut room = Rect::new(
        at,
        line.y,
        (card.right() - ctx.tokens.sm - at).max(0.0),
        line.h,
    );
    let faint = ctx.styles.small(Role::Faint);
    let age = ago(head.at.age_at(ctx.now));
    Label::new(&age, faint).right(ctx, &mut room, ctx.tokens.sm);
    Label::new(&head.lines, faint).right(ctx, &mut room, ctx.tokens.sm);
    let initials = room.take_left(ctx.tokens.xl);
    Label::new(&head.initials, ctx.styles.initials(Role::Muted)).draw(ctx, initials);
    Label::new(&head.author, ctx.styles.label(role)).left(ctx, &mut room, ctx.tokens.sm);
    Label::new(head.source, faint).left(ctx, &mut room, ctx.tokens.sm);
    let Some((label, decorations)) = head.label.as_ref() else {
        return;
    };
    let mut at = Badge::new(label, label_role(label))
        .at(ctx, line, room.x)
        .right();
    for decoration in decorations {
        at = Badge::new(decoration, Role::Faint)
            .at(ctx, line, at + ctx.tokens.xs)
            .right();
    }
}

/// A row of room inside the card, a rule across it half way down when it parts two blocks.
pub(super) fn padded(
    ctx: &mut Ctx,
    line: Rect,
    room: Room,
    (gutter, edges): (Block, (bool, bool)),
) {
    let card = slotted(ctx, line, gutter);
    card_row(ctx, card, edges);
    if room == Room::Between {
        let half = card.h / 2.0;
        let lower = Rect::new(card.x, card.y + half, card.w, half);
        top_rule(ctx, lower, ctx.styles.line());
    }
}

/// A note's row on the deepest ground, and where its card stands on it.
pub(super) fn slotted(ctx: &mut Ctx, line: Rect, gutter: Block) -> Rect {
    groove_ui_kit::shape::ground(ctx, line, Ground::Deep);
    card(ctx, line, gutter)
}

/// Where a note's card stands on one of its rows: from just before the code to short of the end.
fn card(ctx: &mut Ctx, line: Rect, gutter: Block) -> Rect {
    let left = gutter.content(ctx, line) - ctx.tokens.sm;
    let right = line.right() - ctx.tokens.md;
    Rect::new(left, line.y, (right - left).max(0.0), line.h)
}

/// Who said it and which lines, at the card's end; returns where they start.
fn by(ctx: &mut Ctx, card: Rect, said: Noted<'_>) -> f32 {
    let style = ctx.styles.small(Role::Faint);
    let mut at = card.right() - ctx.tokens.sm;
    for text in [said.author, said.lines] {
        if text.is_empty() {
            continue;
        }
        let width = ctx.measure(text, &style);
        at -= width;
        row(ctx, Rect::new(at, card.y, width, card.h), 0.0, text, style);
        at -= ctx.tokens.sm;
    }
    at
}

/// What a note offers, as buttons from the column its words start on.
pub(super) fn acts(
    ctx: &mut Ctx,
    line: Rect,
    acting: &Acting,
    (gutter, edges): (Block, (bool, bool)),
) {
    let card = slotted(ctx, line, gutter);
    card_row(ctx, card, edges);
    let at = gutter.content(ctx, line);
    let mut room = Rect::new(
        at,
        line.y,
        (card.right() - ctx.tokens.sm - at).max(0.0),
        line.h,
    );
    let gap = ctx.tokens.sm;
    if acting.thread {
        acted(ctx, acting, NoteButton::Resolve).right(ctx, &mut room, gap);
        return reply_field(ctx, room, acting);
    }
    acted(ctx, acting, NoteButton::Delete).right(ctx, &mut room, gap);
    let first = acting.post.then_some(NoteButton::Post);
    for button in first
        .into_iter()
        .chain([NoteButton::Edit, NoteButton::Resolve])
    {
        acted(ctx, acting, button).left(ctx, &mut room, gap);
    }
}

/// Where a thread is answered: a field that starts the reply on a click.
fn reply_field(ctx: &mut Ctx, room: Rect, acting: &Acting) {
    groove_ui_kit::shape::ground(ctx, room, Ground::Deep);
    let pad = ctx.tokens.sm;
    let words = Rect::new(room.x + pad, room.y, (room.w - pad).max(0.0), room.h);
    Label::new("Reply…", ctx.styles.body(Role::Faint)).draw(ctx, words);
    ctx.hit(room, Target::Note(acting.origin.clone(), NoteButton::Reply));
}

/// One button of a note's row; delete reads in red on a dimmed red ground.
fn acted(ctx: &Ctx, acting: &Acting, button: NoteButton) -> Button<'static, Target> {
    let label = button.label(acting.resolved);
    let target = Target::Note(acting.origin.clone(), button);
    let (role, ground) = match button {
        NoteButton::Delete => (Role::Bad, ctx.styles.tint(Role::Bad)),
        _ => (Role::Muted, ctx.styles.raised()),
    };
    Button::new(label, target, role, ground).hover(ctx.styles.action())
}
