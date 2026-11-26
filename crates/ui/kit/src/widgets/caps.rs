//! Words in small capitals, a key's name on its cap, and an answer on its key.

use groove_gfx::{Align, Rect};

use crate::base::ctx::{App, Ctx};
use crate::base::style::Role;
use crate::shape::{Panel, from_left, from_right};
use crate::text::Label;
use crate::widgets::Button;

/// `word` in small capitals at the left of `room`.
pub fn cap<A: App>(ctx: &mut Ctx<'_, A>, room: &mut Rect, word: &str, role: Role) -> Rect {
    let word = word.to_uppercase();
    Label::new(&word, ctx.styles.tag(role)).left(ctx, room, ctx.tokens.sm)
}

/// `word` in small capitals at the right of `room`.
pub fn cap_right<A: App>(ctx: &mut Ctx<'_, A>, room: &mut Rect, word: &str, role: Role) -> Rect {
    let word = word.to_uppercase();
    Label::new(&word, ctx.styles.tag(role)).right(ctx, room, ctx.tokens.sm)
}

/// A key's name on a hairline cap at the right of `room`.
pub fn keycap<A: App>(ctx: &mut Ctx<'_, A>, room: &mut Rect, key: &str) {
    capped(ctx, room, key, false);
}

/// The same at the left of `room`.
pub fn keycap_left<A: App>(ctx: &mut Ctx<'_, A>, room: &mut Rect, key: &str) {
    capped(ctx, room, key, true);
}

fn capped<A: App>(ctx: &mut Ctx<'_, A>, room: &mut Rect, key: &str, left: bool) {
    let style = ctx.styles.tag(Role::Faint);
    let wide = ctx.measure(key, &style) + ctx.tokens.sm;
    let slot = match left {
        true => from_left(room, wide, ctx.tokens.xs),
        false => from_right(room, wide, ctx.tokens.sm),
    };
    let cap = slot.align((wide, ctx.tokens.line), Align::Center, Align::Center);
    Panel::default().border(ctx.styles.line()).draw(ctx, cap);
    let word = cap.align((wide - ctx.tokens.sm, cap.h), Align::Center, Align::Center);
    Label::new(key, style).draw(ctx, word);
}

/// One answer at the right of `room`: its word and its key's cap; the first in the ask's colour.
pub fn answer<A: App>(
    ctx: &mut Ctx<'_, A>,
    room: &mut Rect,
    (word, key): (&str, &str),
    (target, first): (A::Target, bool),
) {
    keycap(ctx, room, key);
    let (role, ground) = match first {
        true => (Role::Attention, ctx.styles.band_tint(Role::Attention)),
        false => (Role::Text, ctx.styles.hover()),
    };
    let button = Button::new(word, target, role, ground);
    let edge = if first {
        ctx.styles.band_edge(role)
    } else {
        ctx.styles.line()
    };
    button.edged(edge).squared().right(ctx, room, ctx.tokens.md);
}
