//! The primitives every view is made of; a widget never knows `AppState`.

mod badge;
mod button;
pub mod code;
mod completions;
mod counts;
mod field;
mod fold;
mod heading;
mod icon;
mod input;
mod inset;
mod list;
mod menu;
mod modal;
mod pane;
mod popup;
mod question;
mod scrolled;
mod search;
mod table;
mod tabs;
mod terminal;
mod toggle;
mod typed;

pub use badge::Badge;
pub use button::{Button, Text, picker, slot_at};
pub use completions::{Offer, completions};
pub use counts::{changes, counts, counts_room, faults, faults_room};
pub use field::Field;
pub use fold::{caret, fold, fold_turn, folder};
pub use heading::Heading;
pub use icon::{busy, dot, icon, lead, turned};
pub use input::{Lead, caret_after, input_at, stepped_input, typed_line};
pub use inset::{glass_chosen, glass_hoverable, glass_lit, inset_row, rounded_slice};
pub use list::{Row, glass_list, list};
pub use menu::{glass_menu, glass_size, menu, size as menu_size};
pub use modal::{
    Corner, Material, floating_pane, glass_pane, glass_pill, ground_under, modal, panel_at,
};
pub use pane::{Side, glass_aside, pane};
pub use popup::{Piece, Said, popup};
pub use question::{offer, question};
pub use scrolled::scrolled;
pub use search::Search;
pub use table::{Act, Band, Cell, Column, Laid, Line, Rows, Shown, Slot, Sorted, Table, Width};
pub use tabs::{Tab, glass_tabs, tabs};
pub use terminal::grid_of;
pub use terminal::screen;
pub use toggle::Toggle;
pub use typed::{caret_bar, text_band, wavy};
