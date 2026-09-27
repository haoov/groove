//! The primitives every view is made of; a widget never knows `AppState`.

mod button;
mod counts;
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
pub use counts::{changes, counts, counts_room};
pub use field::Field;
pub use icon::icon;
pub use input::input;
pub use list::{Row, list};
pub use menu::{menu, size as menu_size};
pub use modal::{Corner, modal, panel_at};
pub use picker::picker;
pub use scrolled::scrolled;
pub use tabs::tabs;
pub use terminal::grid_of;
pub use terminal::screen;
