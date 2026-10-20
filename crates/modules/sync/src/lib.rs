//! A lock no panic poisons, and a line of tasks run in the order asked.

mod lock;
mod turn;

#[cfg(test)]
mod tests;

pub use lock::lock;
pub use turn::{Ended, Turn};
