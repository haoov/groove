//! The primitives every view is made of; a widget never knows `AppState`.

mod badge;
mod button;
mod caps;
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
mod meter;
mod modal;
mod neurons;
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
pub use caps::{answer, cap, cap_right, keycap, keycap_left};
pub use completions::{Offer, completions};
pub use counts::{changes, changes_room, counts, counts_room, faults, faults_room};
pub use field::Field;
pub use fold::{caret, fold, fold_turn, folder};
pub use heading::Heading;
pub use icon::{busy, dot, icon, lead, turned};
pub use input::{Lead, caret_after, input_at, stepped_input, typed_line};
pub use inset::{
    card_inset, flat_ground, flat_row, focus_ring, glass_chosen, glass_field, glass_hoverable,
    glass_lit, glass_tinted, inset_row, rounded_slice, sunk_card_row,
};
pub use list::{Row, glass_list, list};
pub use menu::{glass_menu, glass_size};
pub use meter::meter;
pub use modal::{
    Corner, Material, floating_pane, glass_button, glass_pane, glass_pill, ground_lattice,
    ground_under, modal, panel_at, panel_head, solid_pane, sunk_pane,
};
pub use neurons::{NEURON_MS, neurons};
pub use pane::{Side, aside_bounds, glass_aside};
pub use popup::{Piece, Said, popup};
pub use question::{Steps, question};
pub use scrolled::{scrollbar, scrolled};
pub use search::Search;
pub use table::{Act, Band, Cell, Column, Laid, Line, Rows, Shown, Slot, Sorted, Table, Width};
pub use tabs::{Tab, glass_tabs, group_width, marked_glass_tabs, tab_group};
pub use terminal::grid_of;
pub use terminal::{accent_screen, screen};
pub use toggle::Toggle;
pub use typed::{caret_bar, text_band, wavy};
