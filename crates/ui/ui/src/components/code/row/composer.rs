//! The row under a reply being typed: send and cancel.

use groove_gfx::Rect;

use super::note::card_across;
use crate::ctx::Ctx;
use crate::hit::{Composing, Target};
use groove_ui_kit::base::style::Role;
use groove_ui_kit::widgets::Button;
use groove_ui_kit::widgets::code::Block;

/// Send and cancel at the end of the card's row.
pub(super) fn composer(ctx: &mut Ctx, line: Rect, gutter: Block) {
    let card = card_across(ctx, line, gutter);
    let at = gutter.content(ctx, line);
    let mut room = groove_ui_kit::shape::span(line, at, card.right() - ctx.tokens.sm - at);
    let hover = ctx.styles.glass_hover();
    for (label, asked) in [("send", Composing::Save), ("cancel", Composing::Cancel)] {
        let button = Button::new(label, Target::Compose(asked), Role::Muted, hover);
        button
            .flat()
            .hover(hover)
            .right(ctx, &mut room, ctx.tokens.sm);
    }
}
