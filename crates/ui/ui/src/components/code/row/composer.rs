//! The composer around a note being typed: its label chips, its decorations, save and cancel.

use groove_gfx::Rect;

use super::super::{Composed, label_role};
use super::note::slotted;
use crate::ctx::Ctx;
use crate::hit::{Composing, Target};
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::card_row;
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::code::Block;
use groove_ui_kit::widgets::{Badge, Button};

/// The labels the composer offers, and the decorations after them.
const CHIPS: [&str; 7] = [
    "praise",
    "nitpick",
    "issue",
    "suggestion",
    "question",
    "thought",
    "todo",
];
const DECORATIONS: [&str; 2] = ["blocking", "non-blocking"];

/// A row of the composer: the lines and the chips to pick, or the hint and its buttons.
pub(super) fn composer(
    ctx: &mut Ctx,
    line: Rect,
    composed: &Composed,
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
    match composed {
        Composed::Chips { lines, label } => {
            let faint = ctx.styles.small(Role::Faint);
            Label::new(lines, faint).left(ctx, &mut room, ctx.tokens.sm);
            for one in CHIPS {
                let on = label.as_deref() == Some(one);
                let role = if on { label_role(one) } else { Role::Faint };
                chip(
                    ctx,
                    (line, &mut room),
                    Badge::new(one, role),
                    Composing::Label(one),
                );
            }
        }
        Composed::Footer {
            hint,
            save,
            decorations,
        } => footer(ctx, (line, room), (hint, save), decorations.as_deref()),
    }
}

/// Save and cancel at the end; the decorations once a label is chosen; the hint.
fn footer(
    ctx: &mut Ctx,
    (line, mut room): (Rect, Rect),
    (hint, save): (&str, &'static str),
    decorations: Option<&[String]>,
) {
    let (raised, action) = (ctx.styles.raised(), ctx.styles.action());
    for (label, asked) in [(save, Composing::Save), ("cancel", Composing::Cancel)] {
        let button = Button::new(label, Target::Compose(asked), Role::Muted, raised);
        button.hover(action).right(ctx, &mut room, ctx.tokens.sm);
    }
    if let Some(held) = decorations {
        for one in DECORATIONS {
            let role = if held.iter().any(|held| held == one) {
                Role::Text
            } else {
                Role::Faint
            };
            chip(
                ctx,
                (line, &mut room),
                Badge::new(one, role),
                Composing::Decoration(one),
            );
        }
    }
    Label::new(hint, ctx.styles.small(Role::Faint)).draw(ctx, room);
}

/// One chip at the start of `room`, while it has room for it.
fn chip(ctx: &mut Ctx, (line, room): (Rect, &mut Rect), badge: Badge<'_>, asked: Composing) {
    let gap = ctx.tokens.xs;
    if badge.width(ctx) > room.w {
        return;
    }
    let box_ = badge.at(ctx, line, room.x);
    ctx.hit(box_, Target::Compose(asked));
    room.take_left(box_.w + gap);
}
