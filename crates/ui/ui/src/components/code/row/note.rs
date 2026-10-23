//! A note's own rows: what it says, and what it offers.

use groove_gfx::{Edges, Rect};

use super::super::{Acting, Header, Line, Noted, Room, label_role};
use crate::ctx::Ctx;
use crate::hit::{NoteButton, Target};
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::{band, from_left, span, top_rule};
use groove_ui_kit::text::{Label, ago, elide, row};
use groove_ui_kit::widgets::code::Block;
use groove_ui_kit::widgets::{Badge, Button, rounded_slice, typed_line};

/// One row of a note: its part of the card, its mark, its author, its words.
pub(super) fn note(ctx: &mut Ctx, line: Rect, code: &Line<'_>, said: Noted<'_>, gutter: Block) {
    let card = slotted(ctx, line, gutter, code.card);
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
        let rect = span(line, at, by - at - ctx.tokens.sm);
        let link = |url: &str| Target::Link(url.to_string());
        return groove_ui_kit::markdown::row(ctx, rect, prose, role, &link);
    }
    let rect = span(line, at, by - at - ctx.tokens.sm);
    let field = span(line, at - ctx.tokens.xs, card.right() - ctx.tokens.sm - at);
    if let Some(edge) = said.field {
        let ring = edge.lit.then(|| ctx.styles.color(Role::Accent));
        let ground = (ctx.styles.glass_hover(), ring);
        rounded_slice(ctx, field, (edge.first, edge.last), ground);
    }
    let text = elide(ctx, code.text, &words, rect.w);
    typed_line(ctx, rect, (&text, words), (code.caret, code.held));
}

/// A comment's header from the left: initials, author, label, source, lines, age.
fn header(ctx: &mut Ctx, (line, card): (Rect, Rect), gutter: Block, head: &Header, role: Role) {
    if head.reply {
        let rule = ctx.styles.line();
        top_rule(
            ctx,
            card.pad(Edges::across(ctx.tokens.md, ctx.tokens.md)),
            rule,
        );
    }
    let at = gutter.content(ctx, line);
    let mut room = span(line, at, card.right() - ctx.tokens.sm - at);
    let (sm, faint) = (ctx.tokens.sm, ctx.styles.small(Role::Faint));
    let initials = from_left(&mut room, ctx.tokens.xl, 0.0);
    Label::new(&head.initials, ctx.styles.initials(Role::Muted)).draw(ctx, initials);
    Label::new(&head.author, ctx.styles.label(role)).left(ctx, &mut room, sm);
    if let Some((label, decorations)) = head.label.as_ref() {
        Badge::new(label, label_role(label)).left(ctx, &mut room, ctx.tokens.xs);
        for decoration in decorations {
            Badge::new(decoration, Role::Faint).left(ctx, &mut room, ctx.tokens.xs);
        }
        from_left(&mut room, ctx.tokens.xs, 0.0);
    }
    let age = ago(head.at.age_at(ctx.now));
    for said in [head.source, head.lines.as_str(), age.as_str()] {
        Label::new(said, faint).left(ctx, &mut room, sm);
    }
}

/// A row of room inside the card, a rule across it half way down when it parts two blocks.
pub(super) fn padded(ctx: &mut Ctx, line: Rect, room: Room, code: &Line<'_>, gutter: Block) {
    let half = line.h / 2.0;
    match room {
        Room::Over => {
            slotted(ctx, band(line, half, half), gutter, (true, false));
            return;
        }
        Room::Under => {
            slotted(ctx, band(line, 0.0, half), gutter, (false, true));
            return;
        }
        Room::Gap | Room::Between => {}
    }
    let card = slotted(ctx, line, gutter, code.card);
    if room == Room::Between {
        let half = card.h / 2.0;
        let lower = band(card, half, half).pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
        top_rule(ctx, lower, ctx.styles.line());
    }
}

/// Where a note's card stands on its row, its slice of the card drawn there.
pub(super) fn slotted(ctx: &mut Ctx, line: Rect, gutter: Block, edges: (bool, bool)) -> Rect {
    let card = card(ctx, line, gutter);
    rounded_slice(ctx, card, edges, (ctx.styles.glass_card(), None));
    card
}

/// Where a note's card stands on one of its rows: from just before the code to short of the end.
fn card(ctx: &mut Ctx, line: Rect, gutter: Block) -> Rect {
    let left = gutter.content(ctx, line) - ctx.tokens.sm;
    let right = line.right() - ctx.tokens.md;
    span(line, left, right - left)
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
        row(ctx, span(card, at, width), 0.0, text, style);
        at -= ctx.tokens.sm;
    }
    at
}

/// What a note offers, as buttons from the column its words start on.
pub(super) fn acts(
    ctx: &mut Ctx,
    line: Rect,
    (acting, edges): (&Acting, (bool, bool)),
    gutter: Block,
) {
    let card = slotted(ctx, line, gutter, edges);
    let at = gutter.content(ctx, line);
    let mut room = span(line, at, card.right() - ctx.tokens.sm - at);
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
    rounded_slice(ctx, room, (true, true), (ctx.styles.glass_hover(), None));
    let pad = ctx.tokens.sm;
    let words = room.pad(Edges::across(pad, 0.0));
    Label::new("Reply…", ctx.styles.body(Role::Faint)).draw(ctx, words);
    ctx.hit(room, Target::Note(acting.origin.clone(), NoteButton::Reply));
}

/// One button of a note's row; delete reads in red on a dimmed red ground.
fn acted(ctx: &Ctx, acting: &Acting, button: NoteButton) -> Button<'static, Target> {
    let label = button.label(acting.resolved);
    let target = Target::Note(acting.origin.clone(), button);
    let (role, hover) = match button {
        NoteButton::Delete => (Role::Bad, ctx.styles.tint(Role::Bad)),
        _ => (Role::Muted, ctx.styles.glass_hover()),
    };
    Button::new(label, target, role, hover).flat().hover(hover)
}
