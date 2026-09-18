//! What the right button offers on a file: the actions a click on the row does not.

use crate::ctx::Ctx;
use crate::hit::Target;
use crate::widget::menu;
use crate::{Menu, Ui};

/// The actions of one file, in the order they are offered.
pub const ROWS: [&str; 1] = ["discard changes"];

pub fn draw(ctx: &mut Ctx, ui: &Ui, open: &Menu) {
    let within = ctx.layout.window;
    let hovered = match ui.hover {
        Some(Target::MenuRow(at)) => Some(at),
        _ => None,
    };
    let _ = &open.path;
    ctx.layer();
    menu(ctx, open.at, within, &ROWS, hovered);
}
