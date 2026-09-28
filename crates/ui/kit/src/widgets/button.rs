//! A word a click acts on, at the end of a row.

use groove_gfx::{Align, Color, Rect, TextStyle};

use crate::base::ctx::{App, Ctx};
use crate::base::mark::Mark;
use crate::base::style::Role;
use crate::shape::{Panel, box_in};
use crate::text::row;

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

/// The size a button's text is drawn at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Text {
    Small,
    Label,
    Body,
}

/// A word on a bordered ground that a click acts on, in `lit` while the pointer rests on it.
pub struct Button<'a, T> {
    pub label: &'a str,
    pub target: T,
    pub role: Role,
    pub lit: Role,
    pub text: Text,
    /// The ground at rest, none for a flat button, and the one under the pointer.
    pub ground: Option<Color>,
    pub hover: Option<Color>,
    /// Drawn as under the pointer, while what it opens stands open.
    pub open: bool,
    /// A caret after the word, at this turn, for a button that opens a choice.
    pub caret: Option<u8>,
    /// Drawn, but not a target: an action that cannot run now.
    pub inert: bool,
}

impl<'a, T: Clone + PartialEq> Button<'a, T> {
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
            text: Text::Small,
            ground: Some(ground),
            hover: None,
            open: false,
            caret: None,
            inert: false,
        }
    }

    pub fn lit(mut self, role: Role) -> Self {
        self.lit = role;
        self
    }

    pub fn text(mut self, text: Text) -> Self {
        self.text = text;
        self
    }

    pub fn hover(mut self, ground: Color) -> Self {
        self.hover = Some(ground);
        self
    }

    /// No ground at rest; `hover` still shows under the pointer.
    pub fn flat(mut self) -> Self {
        self.ground = None;
        self
    }

    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    pub fn caret(mut self, turn: u8) -> Self {
        self.caret = Some(turn);
        self
    }

    pub fn inert(mut self, inert: bool) -> Self {
        self.inert = inert;
        self
    }

    /// Stands at `x` in `line`; returns its box.
    pub fn at<A: App<Target = T>>(self, ctx: &mut Ctx<'_, A>, line: Rect, x: f32) -> Rect {
        self.draw(ctx, line, x)
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
        let style = self.style(ctx);
        let width = self.content(ctx, &style) + ctx.tokens.sm * 2.0;
        let box_ = self.draw(ctx, *room, room.right() - width);
        room.take_right(box_.w + gap);
        box_
    }

    fn lit_now<A: App<Target = T>>(&self, ctx: &Ctx<'_, A>) -> bool {
        self.open || (!self.inert && ctx.hovered(&self.target))
    }

    fn style<A: App<Target = T>>(&self, ctx: &Ctx<'_, A>) -> TextStyle {
        let role = if self.lit_now(ctx) {
            self.lit
        } else {
            self.role
        };
        match self.text {
            Text::Small => ctx.styles.small(role),
            Text::Label => ctx.styles.label(role),
            Text::Body => ctx.styles.body(role),
        }
    }

    fn content<A: App<Target = T>>(&self, ctx: &mut Ctx<'_, A>, style: &TextStyle) -> f32 {
        let word = ctx.measure(self.label, style);
        match self.caret {
            Some(_) => word + ctx.tokens.xs + style.size,
            None => word,
        }
    }

    fn draw<A: App<Target = T>>(self, ctx: &mut Ctx<'_, A>, line: Rect, x: f32) -> Rect {
        let style = self.style(ctx);
        let content = self.content(ctx, &style);
        let ground = match self.lit_now(ctx) {
            true => self.hover.or(self.ground),
            false => self.ground,
        };
        let box_ = slot_at(ctx, line, x, content, ground);
        row(ctx, box_, ctx.tokens.sm, self.label, style);
        if let Some(turn) = self.caret {
            let mark = box_in(box_, box_.right() - ctx.tokens.sm - style.size, style.size);
            ctx.icon(mark, Mark::Down, turn, ctx.styles.color(Role::Faint));
        }
        if !self.inert {
            ctx.hit(box_, self.target);
        }
        box_
    }
}

/// A value the user can change: a body-text button with a caret, lit while its choice is open.
pub fn picker<'a, T: Clone + PartialEq>(
    label: &'a str,
    target: T,
    role: Role,
    band: Color,
    hover: Color,
) -> Button<'a, T> {
    Button::new(label, target, role, band)
        .text(Text::Body)
        .hover(hover)
        .caret(0)
}
