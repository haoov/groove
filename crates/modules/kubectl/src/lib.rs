//! The kubectl reads the agent asks Groove for, checked before they run.

mod check;
mod flags;

#[cfg(test)]
mod tests;

pub use check::{Checked, check};
