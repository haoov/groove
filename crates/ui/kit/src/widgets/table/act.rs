//! What a cell holds in place of its text: a button or a toggle.

use groove_gfx::Rect;

use crate::base::ctx::{App, Ctx};
use crate::base::style::Role;
use crate::widgets::{Button, Toggle};

pub enum Act<'a, T> {
    Button(&'a str, T),
    Toggle(bool, T),
    /// A mark in its own colour.
    Mark(crate::base::mark::Mark, Role, T),
}

impl<T: Clone + PartialEq> Act<'_, T> {
    /// Draws the button or the toggle at the start of `rect`.
    pub(super) fn draw<A: App<Target = T>>(self, ctx: &mut Ctx<'_, A>, rect: Rect) {
        let mut room = rect;
        match self {
            Act::Button(label, target) => {
                let ground = ctx.styles.ground();
                Button::new(label, target, Role::Muted, ground).left(ctx, &mut room, 0.0);
            }
            Act::Toggle(on, target) => {
                Toggle::new(on, target).left(ctx, &mut room, 0.0);
            }
            Act::Mark(mark, role, target) => {
                let hover = ctx.styles.hover();
                Button::icon(mark, 0, target, role)
                    .hover(hover)
                    .left(ctx, &mut room, 0.0);
            }
        }
    }
}
