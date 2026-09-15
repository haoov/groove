//! The primitives every view is made of. A widget draws into a rect and knows the
//! context; it never knows `AppState`.

mod counts;
mod icon;
mod input;
mod list;
mod modal;
mod picker;
mod tabs;
mod terminal;
mod text;
mod time;

pub use counts::counts;
pub use icon::{after_mark, box_in, icon, leading, turn};
pub use input::input;
pub use list::{Row, list};
pub use modal::modal;
pub use picker::{divider, picker};
pub use tabs::tabs;
#[cfg(test)]
pub(crate) use terminal::grid_of;
pub use terminal::screen;
pub use text::{elide, hairline, row};
pub use time::ago;
