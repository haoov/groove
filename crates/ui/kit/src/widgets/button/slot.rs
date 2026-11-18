//! A button's bordered slot, and the sizes its word is drawn at.

use groove_gfx::{Color, Rect, TextStyle};

use crate::base::ctx::{App, Ctx};
use crate::base::style::Role;
use crate::shape::Panel;

/// Room of `content` plus the padding either side, from `x`, on a bordered `ground`.
pub fn slot_at<A: App>(
    ctx: &mut Ctx<'_, A>,
    line: Rect,
    x: f32,
    content: f32,
    ground: Option<Color>,
) -> Rect {
    let pad = ctx.tokens.sm;
    let height = (ctx.tokens.row - ctx.tokens.xs).min(line.h - ctx.tokens.xs);
    let box_ = Rect::new(
        x,
        line.y + (line.h - height) / 2.0,
        content + pad * 2.0,
        height,
    );
    if let Some(ground) = ground {
        let line = ctx.styles.line();
        Panel::default()
            .ground(ground)
            .border(line)
            .radius(ctx.tokens.round)
            .draw(ctx, box_);
    }
    box_
}

/// The size a button's text is drawn at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Text {
    Small,
    Label,
    Body,
}

impl Text {
    /// The style this size draws in, in `role`.
    pub fn style(self, styles: &crate::base::style::Styles, role: Role) -> TextStyle {
        match self {
            Text::Small => styles.small(role),
            Text::Label => styles.label(role),
            Text::Body => styles.body(role),
        }
    }
}
