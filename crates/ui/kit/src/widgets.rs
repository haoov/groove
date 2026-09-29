//! The primitives every view is made of; a widget never knows `AppState`.

mod badge;
mod button;
mod counts;
mod field;
mod fold;
mod icon;
mod input;
mod list;
mod menu;
mod modal;
mod scrolled;
mod search;
mod tabs;
mod terminal;

pub use badge::Badge;
pub use button::{Button, Text, picker, slot_at};
pub use counts::{changes, counts, counts_room};
pub use field::Field;
pub use fold::fold;
pub use icon::icon;
pub use input::input;
pub use list::{Row, list};
pub use menu::{menu, size as menu_size};
pub use modal::{Corner, modal, panel_at};
pub use scrolled::scrolled;
pub use search::Search;
pub use tabs::{Tab, tabs};
pub use terminal::grid_of;
pub use terminal::screen;
