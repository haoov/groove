//! Pieces made of widgets that know what Groove holds: code, a delivery, a worktree.

mod code;
mod delivery;
pub mod worktree_row;

pub use code::{
    Acting, Gutters, Line, Noted, Rows, chars_of, code, code_at, first, head_mark, height, visible,
};
pub use delivery::{delivered, room_for};
