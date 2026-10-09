//! A value a click copies, and a link ctrl+click opens: on hover, a ground and the copy mark.

use groove_gfx::{Align, Rect};
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, row_in};
use groove_ui_kit::shape::Panel;
use groove_ui_kit::text::Label;

use super::super::opened::Link;
use crate::ctx::Ctx;
use crate::hit::Target;

/// One value, what the tab copied last, and the object a ctrl+click on it opens.
pub(super) struct Copied<'a> {
    text: &'a str,
    role: Role,
    last: Option<&'a str>,
    open: Option<Link>,
    copies: &'a str,
}

impl<'a> Copied<'a> {
    pub(super) fn new(text: &'a str, role: Role, last: Option<&'a str>) -> Self {
        Self {
            text,
            role,
            last,
            open: None,
            copies: text,
        }
    }

    pub(super) fn open(mut self, open: Option<Link>) -> Self {
        self.open = open;
        self
    }

    /// Copies `part` of the text shown.
    pub(super) fn copies(mut self, part: &'a str) -> Self {
        self.copies = part;
        self
    }

    /// Draws in `rect`, keeping room for the mark at the text's end; returns the box the text took.
    pub(super) fn draw(self, ctx: &mut Ctx, rect: Rect) -> Rect {
        let (icon, gap) = (ctx.tokens.small, ctx.tokens.xs);
        let room = Rect {
            w: (rect.w - icon - gap).max(0.0),
            ..rect
        };
        let style = ctx.styles.small(self.role);
        let wide = ctx.measure(self.text, &style).min(room.w);
        let text = Rect { w: wide, ..rect };
        let mark = Rect {
            x: text.right() + gap,
            w: icon,
            ..rect
        };
        let spot = Rect {
            w: wide + gap + icon,
            ..rect
        };
        let link = self.open.map(Box::new);
        let on = ctx.interact(spot, Target::ResourceCopy(self.copies.to_string(), link));
        if on {
            let ground = Panel::default().ground(ctx.styles.hover());
            ground.radius(ctx.tokens.round).draw(ctx, spot);
        }
        let drawn = Label::new(self.text, style).draw(ctx, room);
        if on {
            let (shape, role) = match self.last == Some(self.copies) {
                true => (Mark::Copied, Role::Ok),
                false => (Mark::Copy, Role::Faint),
            };
            let at = mark.align((icon, icon), Align::Center, Align::Center);
            groove_ui_kit::widgets::icon(ctx, at, shape, role);
        }
        drawn
    }

    /// Draws at the left of `room`, which gives up the text, the mark and `gap`.
    pub(super) fn left(self, ctx: &mut Ctx, room: &mut Rect, gap: f32) -> Rect {
        let style = ctx.styles.small(self.role);
        let wide = ctx.measure(self.text, &style) + ctx.tokens.xs + ctx.tokens.small;
        let fixed = |width: f32| Spec::default().width(width);
        let [at, _, rest] = row_in(*room, [fixed(wide.min(room.w)), fixed(gap), Spec::fill()]);
        *room = rest;
        self.draw(ctx, at)
    }
}
