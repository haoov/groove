//! A section's heading: its name in capitals, how many it holds, and the caret that folds it.

use groove_gfx::{Edges, Rect};

use crate::base::ctx::{App, Ctx};
use crate::base::style::Role;
use crate::shape::hoverable;
use crate::text::Label;
use crate::widgets::fold;

pub struct Heading<'a, T> {
    label: &'a str,
    count: Option<usize>,
    /// Whether what it heads is open, when it folds.
    fold: Option<bool>,
    target: Option<T>,
    role: Role,
}

impl<'a, T: Clone + PartialEq> Heading<'a, T> {
    pub fn new(label: &'a str) -> Self {
        Self {
            label,
            count: None,
            fold: None,
            target: None,
            role: Role::Faint,
        }
    }

    /// Shown after the name; nothing when it is zero.
    pub fn count(mut self, count: usize) -> Self {
        self.count = Some(count);
        self
    }

    pub fn fold(mut self, open: bool) -> Self {
        self.fold = Some(open);
        self
    }

    /// What a click on the heading asks for.
    pub fn target(mut self, target: T) -> Self {
        self.target = Some(target);
        self
    }

    pub fn role(mut self, role: Role) -> Self {
        self.role = role;
        self
    }

    /// Across `line`, padded at both ends.
    pub fn draw<A: App<Target = T>>(self, ctx: &mut Ctx<'_, A>, line: Rect) {
        let md = ctx.tokens.md;
        self.within(ctx, line, line.pad(Edges::across(md, md)));
    }

    /// Across `line`, its text in `room`, what the caller has kept of the line.
    pub fn within<A: App<Target = T>>(self, ctx: &mut Ctx<'_, A>, line: Rect, mut room: Rect) {
        if let Some(target) = self.target {
            hoverable(ctx, line, target);
        }
        if let Some(open) = self.fold {
            fold(ctx, &mut room, open, self.role);
        }
        let name = self.label.to_uppercase();
        let text = match self.count {
            None | Some(0) => name,
            Some(n) => format!("{name} · {n}"),
        };
        Label::new(&text, ctx.styles.heading(self.role)).draw(ctx, room);
    }
}
