//! A word a click acts on, at the end of a row.

use groove_gfx::{Align, Color, Rect, TextStyle};

use crate::base::ctx::{App, Ctx};
use crate::base::mark::Mark;
use crate::base::style::Role;
use crate::shape::{Panel, box_in};
use crate::text::row;

/// A label on `ground` at the row's right end; returns its box.
pub fn button<A: App>(
    ctx: &mut Ctx<'_, A>,
    line: Rect,
    label: &str,
    style: TextStyle,
    ground: Option<Color>,
) -> Rect {
    let word = ctx.measure(label, &style);
    let box_ = slot(ctx, line, word, ground);
    row(ctx, box_, ctx.tokens.sm, label, style);
    box_
}

/// Room of `content` plus the padding either side, at the row's right end.
pub fn slot<A: App>(ctx: &mut Ctx<'_, A>, line: Rect, content: f32, ground: Option<Color>) -> Rect {
    let pad = ctx.tokens.sm;
    let at = line.right() - pad - (content + pad * 2.0);
    slot_at(ctx, line, at, content, ground)
}

/// The same room from `x`, on a bordered `ground`; returns its box.
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
        Panel::default().ground(ground).border(line).draw(ctx, box_);
    }
    box_
}

/// A mark at the row's right end, on `grounds.1` while the pointer rests on it.
pub fn mark_button<A: App>(
    ctx: &mut Ctx<'_, A>,
    line: Rect,
    target: A::Target,
    (mark, turn, role): (Mark, u8, Role),
    grounds: (Color, Color),
) -> Rect {
    let ground = match ctx.hovered(&target) {
        true => grounds.1,
        false => grounds.0,
    };
    let size = ctx.tokens.small;
    let box_ = slot(ctx, line, size, Some(ground));
    let at = box_.align((size, size), Align::Center, Align::Center);
    ctx.icon(at, mark, turn, ctx.styles.color(role));
    ctx.hit(box_, target);
    box_
}

/// A word on a bordered ground that a click acts on, in `lit` while the pointer rests on it.
pub struct Word<'a, T> {
    pub label: &'a str,
    pub target: T,
    pub role: Role,
    pub lit: Role,
    pub ground: Color,
    pub caret: bool,
}

impl<'a, T: Clone + PartialEq> Word<'a, T> {
    /// A muted word lights to text; any other keeps its role.
    pub fn new(label: &'a str, target: T, role: Role, ground: Color) -> Self {
        let lit = match role {
            Role::Muted => Role::Text,
            other => other,
        };
        Self {
            label,
            target,
            role,
            lit,
            ground,
            caret: false,
        }
    }

    pub fn lit(mut self, role: Role) -> Self {
        self.lit = role;
        self
    }

    pub fn caret(mut self) -> Self {
        self.caret = true;
        self
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
        let style = ctx.styles.small(self.role(ctx));
        let width = self.content(ctx, &style) + ctx.tokens.sm * 2.0;
        let box_ = self.draw(ctx, *room, room.right() - width);
        room.take_right(box_.w + gap);
        box_
    }

    fn role<A: App<Target = T>>(&self, ctx: &Ctx<'_, A>) -> Role {
        match ctx.hovered(&self.target) {
            true => self.lit,
            false => self.role,
        }
    }

    fn content<A: App<Target = T>>(&self, ctx: &mut Ctx<'_, A>, style: &TextStyle) -> f32 {
        let word = ctx.measure(self.label, style);
        match self.caret {
            true => word + ctx.tokens.xs + ctx.tokens.small,
            false => word,
        }
    }

    fn draw<A: App<Target = T>>(self, ctx: &mut Ctx<'_, A>, line: Rect, x: f32) -> Rect {
        let style = ctx.styles.small(self.role(ctx));
        let content = self.content(ctx, &style);
        let box_ = slot_at(ctx, line, x, content, Some(self.ground));
        row(ctx, box_, ctx.tokens.sm, self.label, style);
        if self.caret {
            let size = ctx.tokens.small;
            let mark = box_in(box_, box_.right() - ctx.tokens.sm - size, size);
            ctx.icon(mark, Mark::Down, Mark::UPWARDS, style.color);
        }
        ctx.hit(box_, self.target);
        box_
    }
}
