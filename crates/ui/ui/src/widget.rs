//! The primitives every view is made of. A widget draws into a rect and knows the
//! context; it never knows `AppState`.

mod button;
mod code;
mod counts;
mod field;
mod icon;
mod input;
mod list;
mod menu;
mod modal;
mod picker;
mod tabs;
mod terminal;
mod text;
mod time;

pub use button::{button, slot};
pub use code::{Gutters, Line, Rows, chars_of, code, code_at, first, head_mark, height, visible};
pub use counts::counts;
pub use field::Field;
pub use icon::{after_mark, box_in, icon, leading, turn};
pub use input::input;
pub use list::{Row, list};
pub use menu::{menu, size as menu_size};
pub use modal::{modal, panel_at};
pub use picker::{divider, picker};
pub use tabs::tabs;
#[cfg(test)]
pub(crate) use terminal::grid_of;
pub use terminal::screen;
pub use text::{elide, elide_start, hairline, row, ruled};
pub use time::ago;
