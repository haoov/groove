//! One cell of a table: a text in a size and a role, a mark before it, or a badge.

use std::borrow::Cow;

use groove_gfx::{Edges, Rect};

use crate::base::ctx::{App, Ctx};
use crate::base::mark::Mark;
use crate::base::style::Role;
use crate::text::{elide, row};
use crate::widgets::Badge;

pub struct Cell<'a> {
    text: Cow<'a, str>,
    role: Role,
    label: bool,
    badge: bool,
    mark: Option<(Mark, u8, Role)>,
}

impl<'a> Cell<'a> {
    /// A text in the small size.
    pub fn small(text: impl Into<Cow<'a, str>>, role: Role) -> Self {
        Self {
            text: text.into(),
            role,
            label: false,
            badge: false,
            mark: None,
        }
    }

    /// A text in the label size, as a title is.
    pub fn label(text: impl Into<Cow<'a, str>>, role: Role) -> Self {
        Self {
            label: true,
            ..Self::small(text, role)
        }
    }

    /// The text as a badge in its role's colour.
    pub fn badge(mut self) -> Self {
        self.badge = true;
        self
    }

    /// A mark before the text: its turn in eighths, and its own role.
    pub fn mark(mut self, mark: Mark, turn: u8, role: Role) -> Self {
        self.mark = Some((mark, turn, role));
        self
    }

    /// Draws in `rect`, at its start or against its end; returns where its text starts and ends.
    pub(super) fn draw<A: App>(self, ctx: &mut Ctx<'_, A>, rect: Rect, end: bool) -> (f32, f32) {
        if self.text.is_empty() {
            return (rect.x, rect.x);
        }
        if self.badge {
            let box_ = Badge::new(&self.text, self.role).at(ctx, rect, rect.x);
            return (box_.x, box_.right());
        }
        let style = match self.label {
            true => ctx.styles.label(self.role),
            false => ctx.styles.small(self.role),
        };
        let mut room = rect;
        if let Some((mark, turn, role)) = self.mark {
            let (size, gap) = match self.label {
                true => (ctx.tokens.icon, ctx.tokens.sm),
                false => (ctx.tokens.small, ctx.tokens.xs),
            };
            let (at, pad) = match end {
                true => (room.take_right(size), Edges::across(0.0, gap)),
                false => (room.take_left(size), Edges::across(gap, 0.0)),
            };
            room = room.pad(pad);
            let box_ = crate::shape::square(at, size);
            crate::widgets::turned(ctx, box_, mark, turn, role);
        }
        let text = elide(ctx, &self.text, &style, room.w);
        let wide = ctx.measure(&text, &style);
        let lead = match end {
            true => (room.w - wide).max(0.0),
            false => 0.0,
        };
        row(ctx, room, lead, &text, style);
        (room.x + lead, room.x + lead + wide)
    }
}
