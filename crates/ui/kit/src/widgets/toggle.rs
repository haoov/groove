//! A toggle: a track whose knob stands at its right end while on, a label before it when given.

use groove_gfx::Rect;

use crate::base::ctx::{App, Ctx};
use crate::base::style::Role;

/// A switch a click flips, its track in `role` while on.
pub struct Toggle<'a, T> {
    pub on: bool,
    pub target: T,
    pub label: Option<&'a str>,
    pub role: Role,
}

impl<'a, T: Clone + PartialEq> Toggle<'a, T> {
    pub fn new(on: bool, target: T) -> Self {
        Self {
            on,
            target,
            label: None,
            role: Role::Accent,
        }
    }

    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    pub fn role(mut self, role: Role) -> Self {
        self.role = role;
        self
    }

    pub fn width<A: App<Target = T>>(&self, ctx: &mut Ctx<'_, A>) -> f32 {
        let (track, _) = track(ctx);
        let Some(label) = self.label else {
            return track;
        };
        let style = ctx.styles.small(Role::Text);
        ctx.measure(label, &style) + ctx.tokens.sm + track
    }

    /// Stands at the left of `room`, which gives up its box and `gap`.
    pub fn left<A: App<Target = T>>(self, ctx: &mut Ctx<'_, A>, room: &mut Rect, gap: f32) -> Rect {
        let box_ = self.draw(ctx, *room, room.x);
        room.take_left(box_.w + gap);
        box_
    }

    /// Stands at the right of `room`, which gives up its box and `gap`.
    pub fn right<A: App<Target = T>>(
        self,
        ctx: &mut Ctx<'_, A>,
        room: &mut Rect,
        gap: f32,
    ) -> Rect {
        let width = self.width(ctx);
        let box_ = self.draw(ctx, *room, room.right() - width);
        room.take_right(box_.w + gap);
        box_
    }

    fn draw<A: App<Target = T>>(self, ctx: &mut Ctx<'_, A>, line: Rect, x: f32) -> Rect {
        let width = self.width(ctx);
        let lit = ctx.hovered(&self.target);
        let box_ = Rect::new(x, line.y, width, line.h);
        let (wide, tall) = track(ctx);
        if let Some(label) = self.label {
            let role = if lit { Role::Text } else { Role::Muted };
            let style = ctx.styles.small(role);
            let room = Rect::new(x, line.y, width - wide - ctx.tokens.sm, line.h);
            crate::text::row(ctx, room, 0.0, label, style);
        }
        let top = (line.y + (line.h - tall) / 2.0).round();
        let track = Rect::new((box_.right() - wide).round(), top, wide, tall);
        let fill = match (self.on, lit) {
            (true, _) => ctx.styles.color(self.role),
            (false, true) => ctx.styles.color(Role::Faint),
            (false, false) => ctx.styles.color(Role::Ghost),
        };
        ctx.rounded(track, fill, tall / 2.0);
        let pad = ctx.tokens.edge * 2.0;
        let knob = tall - pad * 2.0;
        let left = match self.on {
            true => track.right() - pad - knob,
            false => track.x + pad,
        };
        let ground = ctx.styles.ground();
        ctx.rounded(
            Rect::new(left, track.y + pad, knob, knob),
            ground,
            knob / 2.0,
        );
        ctx.hit(box_, self.target);
        box_
    }
}

/// The track's width and height: twice as wide as the small text is tall.
fn track<A: App>(ctx: &Ctx<'_, A>) -> (f32, f32) {
    let tall = ctx.tokens.small.round();
    (tall * 2.0, tall)
}
