//! A lock no panic poisons, a line of tasks run in the order asked, and work off the caller's thread.

mod blocking;
mod lock;
mod turn;

#[cfg(test)]
mod tests;

pub use blocking::blocking;
pub use lock::lock;
pub use turn::{Ended, Turn};
