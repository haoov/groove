use groove_gfx::{Color, Rect, TextStyle};

use crate::base::ctx::{App, Ctx};
use crate::base::mark::Mark;
use crate::base::style::Role;
use crate::shape::{hairline, hoverable};
use crate::text::row;
use crate::widgets::{Button, Text};

/// One tab: its label, raised while selected, and the mark that closes it.
pub struct Tab<'a, T> {
    pub label: &'a str,
    pub target: T,
    pub selected: bool,
    pub text: Text,
    /// The label's role while another tab is up.
    pub quiet: Role,
    /// The selected tab's ground, raised unless said otherwise.
    pub ground: Option<Color>,
    pub tight: bool,
    pub close: Option<(Mark, T)>,
}

impl<'a, T: Clone + PartialEq> Tab<'a, T> {
    pub fn new(label: &'a str, target: T, selected: bool) -> Self {
        Self {
            label,
            target,
            selected,
            text: Text::Label,
            quiet: Role::Muted,
            ground: None,
            tight: false,
            close: None,
        }
    }

    pub fn text(mut self, text: Text) -> Self {
        self.text = text;
        self
    }

    pub fn quiet(mut self, role: Role) -> Self {
        self.quiet = role;
        self
    }

    pub fn ground(mut self, ground: Color) -> Self {
        self.ground = Some(ground);
        self
    }

    /// The small padding, for tabs set in a header's end.
    pub fn tight(mut self) -> Self {
        self.tight = true;
        self
    }

    pub fn close(mut self, mark: Mark, target: T) -> Self {
        self.close = Some((mark, target));
        self
    }

    /// Stands at the left of `room`, which gives up its box and `gap`.
    pub fn left<A: App<Target = T>>(self, ctx: &mut Ctx<'_, A>, room: &mut Rect, gap: f32) -> Rect {
        let box_ = room.take_left(self.width(ctx));
        room.take_left(gap);
        self.draw(ctx, box_);
        box_
    }

    /// Stands at the right of `room`, which gives up its box and `gap`.
    pub fn right<A: App<Target = T>>(
        self,
        ctx: &mut Ctx<'_, A>,
        room: &mut Rect,
        gap: f32,
    ) -> Rect {
        let box_ = room.take_right(self.width(ctx));
        room.take_right(gap);
        self.draw(ctx, box_);
        box_
    }

    fn pad<A: App<Target = T>>(&self, ctx: &Ctx<'_, A>) -> f32 {
        if self.tight {
            ctx.tokens.sm
        } else {
            ctx.tokens.md
        }
    }

    fn style<A: App<Target = T>>(&self, ctx: &Ctx<'_, A>) -> TextStyle {
        let role = if self.selected {
            Role::Text
        } else {
            self.quiet
        };
        match self.text {
            Text::Small => ctx.styles.small(role),
            Text::Label => ctx.styles.label(role),
            Text::Body => ctx.styles.body(role),
        }
    }

    fn width<A: App<Target = T>>(&self, ctx: &mut Ctx<'_, A>) -> f32 {
        let closing = match self.close {
            Some(_) => ctx.tokens.small + ctx.tokens.sm * 2.0,
            None => 0.0,
        };
        let style = self.style(ctx);
        ctx.measure(self.label, &style) + self.pad(ctx) * 2.0 + closing
    }

    /// The label is drawn whole: the tab is as wide as it.
    fn draw<A: App<Target = T>>(self, ctx: &mut Ctx<'_, A>, box_: Rect) {
        if self.selected {
            let ground = self.ground.unwrap_or(ctx.styles.raised());
            ctx.quad(box_, ground);
        }
        hoverable(ctx, box_, self.target.clone());
        let style = self.style(ctx);
        let pad = self.pad(ctx);
        if let Some((mark, target)) = self.close {
            let mut end = box_;
            Button::icon(mark, 0, target, Role::Faint).right(ctx, &mut end, 0.0);
        }
        row(ctx, box_, pad, self.label, style);
    }
}

/// Tabs from the left of `rect`, the selected one raised, a hairline under; returns their boxes.
pub fn tabs<A: App>(
    ctx: &mut Ctx<'_, A>,
    rect: Rect,
    tabs: &[(&str, A::Target)],
    selected: usize,
) -> Vec<Rect> {
    let mut room = rect;
    let boxes = tabs
        .iter()
        .enumerate()
        .map(|(i, (label, target))| {
            Tab::new(label, target.clone(), i == selected).left(ctx, &mut room, 0.0)
        })
        .collect();
    let line = ctx.styles.line();
    hairline(ctx, rect, line);
    boxes
}
