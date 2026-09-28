//! A search field: its glass, what is typed with the caret while the keys are in it, or its hint.

use groove_gfx::{Edges, Rect};

use crate::base::ctx::{App, Ctx};
use crate::base::mark::Mark;
use crate::base::style::Role;
use crate::shape::square;
use crate::text::{Label, row};
use crate::widgets::Field;

pub struct Search<'a, T> {
    pub field: &'a Field,
    pub target: T,
    pub typing: bool,
    pub hint: &'a str,
    /// What stands between the glass and the text, a term's name.
    pub prefix: Option<&'a str>,
    pub code: bool,
    /// Whether the glass is drawn; its room is kept either way.
    pub glass: bool,
    /// The glass's and the hint's role, and the typed text's while the keys are elsewhere.
    pub quiet: Role,
    pub rest: Role,
}

impl<'a, T: Clone + PartialEq> Search<'a, T> {
    pub fn new(field: &'a Field, target: T, typing: bool) -> Self {
        Self {
            field,
            target,
            typing,
            hint: "",
            prefix: None,
            code: false,
            glass: true,
            quiet: Role::Ghost,
            rest: Role::Text,
        }
    }

    pub fn hint(mut self, hint: &'a str) -> Self {
        self.hint = hint;
        self
    }

    pub fn prefix(mut self, prefix: &'a str) -> Self {
        self.prefix = Some(prefix);
        self
    }

    pub fn code(mut self) -> Self {
        self.code = true;
        self
    }

    pub fn glass(mut self, glass: bool) -> Self {
        self.glass = glass;
        self
    }

    /// The glass, the hint and the text at rest all in `role`.
    pub fn faint(mut self, role: Role) -> Self {
        self.quiet = role;
        self.rest = role;
        self
    }

    /// The field across `line`, the whole line its target; returns the room right of the text.
    pub fn draw<A: App<Target = T>>(self, ctx: &mut Ctx<'_, A>, line: Rect) -> Rect {
        ctx.hit(line, self.target.clone());
        let size = ctx.tokens.icon;
        let mut room = line.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
        let glass = square(room.take_left(size), size);
        room.take_left(ctx.tokens.sm);
        if self.glass {
            let role = if self.typing { Role::Text } else { self.quiet };
            ctx.icon(glass, Mark::Search, 0, ctx.styles.color(role));
        }
        if let Some(prefix) = self.prefix {
            let style = ctx.styles.code(Role::Ghost);
            Label::new(prefix, style).left(ctx, &mut room, ctx.tokens.sm);
        }
        let (text, role) = match (self.typing, self.field.is_empty()) {
            (true, _) => (self.field.shown(), Role::Text),
            (false, true) => (self.hint.to_string(), self.quiet),
            (false, false) => (self.field.text().to_string(), self.rest),
        };
        let style = match self.code {
            true => ctx.styles.code(role),
            false => ctx.styles.body(role),
        };
        row(ctx, room, 0.0, &text, style);
        room.take_left(ctx.measure(&text, &style));
        room
    }
}
