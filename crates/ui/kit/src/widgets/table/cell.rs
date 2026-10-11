//! One cell of a table: a text in a size and a role, a mark or a bar before it, or a badge.

use std::borrow::Cow;

use groove_gfx::{Align, Edges, Rect};

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
    bar: Option<Bar>,
}

/// A short track before the text: its fill and a tick, each a part of the whole, in a role.
#[derive(Debug, Clone, Copy)]
struct Bar {
    fill: Option<f32>,
    tick: Option<f32>,
    role: Role,
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
            bar: None,
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

    /// A track before the text, filled to `fill` in `role`, a tick at `tick`.
    pub fn bar(mut self, fill: Option<f32>, tick: Option<f32>, role: Role) -> Self {
        self.bar = Some(Bar { fill, tick, role });
        self
    }

    /// Draws in `rect`, at its start or against its end; returns where its text starts and ends.
    pub(super) fn draw<A: App>(self, ctx: &mut Ctx<'_, A>, rect: Rect, end: bool) -> (f32, f32) {
        if self.text.is_empty() {
            let Some((mark, turn, role)) = self.mark else {
                return (rect.x, rect.x);
            };
            let size = ctx.tokens.small;
            let box_ = rect.align((size, size), Align::Center, Align::Center);
            crate::widgets::turned(ctx, box_, mark, turn, role);
            return (box_.x, box_.right());
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
        if let Some(bar) = self.bar {
            let track = room.take_left(ctx.tokens.aside_near / 2.0);
            room = room.pad(Edges::across(ctx.tokens.sm, 0.0));
            bar.draw(ctx, track);
        }
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

impl Bar {
    fn draw<A: App>(self, ctx: &mut Ctx<'_, A>, rect: Rect) {
        let thick = ctx.tokens.xs * 2.0;
        let track = rect.align((rect.w, thick), Align::Start, Align::Center);
        let round = thick / 2.0;
        ctx.rounded(track, ctx.styles.line(), round);
        if let Some(fill) = self.fill {
            let filled = Rect {
                w: track.w * fill.clamp(0.0, 1.0),
                ..track
            };
            ctx.rounded(filled, ctx.styles.color(self.role), round);
        }
        if let Some(tick) = self.tick {
            let x = track.x + track.w * tick.clamp(0.0, 1.0);
            let tall = thick * 2.0;
            let mark = Rect::new(
                x - ctx.tokens.hairline,
                track.y - thick / 2.0,
                ctx.tokens.hairline * 2.0,
                tall,
            );
            ctx.quad(mark, ctx.styles.color(Role::Text));
        }
    }
}
