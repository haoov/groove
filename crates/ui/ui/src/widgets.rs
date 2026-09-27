//! The primitives every view is made of; a widget never knows `AppState`.

mod button;
mod code;
mod counts;
mod delivery;
mod field;
mod icon;
mod input;
mod list;
mod menu;
mod modal;
mod picker;
mod scrolled;
mod tabs;
mod terminal;

pub use button::{Word, button, mark_button, slot_at};
pub use code::{
    Acting, Gutters, Line, Noted, Rows, chars_of, code, code_at, first, head_mark, height, visible,
};
pub use counts::{counts, counts_room};
pub use delivery::{delivered, room_for};
pub use field::Field;
pub use icon::icon;
pub use input::input;
pub use list::{Row, list};
pub use menu::{menu, size as menu_size};
pub use modal::{modal, panel_at};
pub use picker::picker;
pub use scrolled::scrolled;
pub use tabs::tabs;
#[cfg(test)]
pub(crate) use terminal::grid_of;
pub use terminal::screen;
