//! The primitives every view is made of. A widget draws into a rect and knows the
//! context; it never knows `AppState`.

mod input;
mod list;
mod modal;
mod tabs;
mod terminal;
mod text;

pub use input::input;
pub use list::{Row, list};
pub use modal::modal;
pub use tabs::tabs;
#[cfg(test)]
pub(crate) use terminal::grid_of;
pub use terminal::screen;
pub use text::{hairline, row};
