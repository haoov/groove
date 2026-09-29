//! A badge: a word, and a mark before it when given, on a dim ground and edge of its own colour.

use groove_gfx::Rect;

use crate::base::ctx::{App, Ctx};
use crate::base::mark::Mark;
use crate::base::style::Role;
use crate::base::tokens::{BADGE_EDGE, BADGE_GROUND};
use crate::shape::box_in;
use crate::text::row;

pub struct Badge<'a> {
    pub text: &'a str,
    pub mark: Option<Mark>,
    pub role: Role,
}

impl<'a> Badge<'a> {
    pub fn new(text: &'a str, role: Role) -> Self {
        Self {
            text,
            mark: None,
            role,
        }
    }

    pub fn mark(mut self, mark: Mark) -> Self {
        self.mark = Some(mark);
        self
    }

    pub fn width<A: App>(&self, ctx: &mut Ctx<'_, A>) -> f32 {
        let style = ctx.styles.small(self.role);
        let mark = self.mark.map_or(0.0, |_| style.size + ctx.tokens.xs);
        ctx.measure(self.text, &style) + mark + ctx.tokens.xs * 2.0
    }

    /// Stands at `x`, centred down `line`; returns its box.
    pub fn at<A: App>(self, ctx: &mut Ctx<'_, A>, line: Rect, x: f32) -> Rect {
        let style = ctx.styles.small(self.role);
        let colour = style.color;
        let (wide, tall) = (
            self.width(ctx),
            (style.size + ctx.tokens.xs * 2.0).min(line.h),
        );
        let top = line.y + (line.h - tall) / 2.0;
        let box_ = Rect::new(x.round(), top.round(), wide.round(), tall.round());
        let corner = ctx.tokens.corner;
        ctx.rounded(box_, colour.with_alpha(BADGE_GROUND), corner);
        let edge = ctx.tokens.edge;
        ctx.ring(box_, colour.with_alpha(BADGE_EDGE), corner, edge);
        let mut at = box_.x + ctx.tokens.xs;
        if let Some(mark) = self.mark {
            ctx.icon(box_in(box_, at, style.size), mark, 0, colour);
            at += style.size + ctx.tokens.xs;
        }
        row(
            ctx,
            Rect::new(at, box_.y, box_.right() - at, tall),
            0.0,
            self.text,
            style,
        );
        box_
    }

    /// Stands at the right of `room`, which gives up its box and `gap`.
    pub fn right<A: App>(self, ctx: &mut Ctx<'_, A>, room: &mut Rect, gap: f32) -> Rect {
        let wide = self.width(ctx);
        let box_ = self.at(ctx, *room, room.right() - wide);
        room.take_right(wide + gap);
        box_
    }
}
