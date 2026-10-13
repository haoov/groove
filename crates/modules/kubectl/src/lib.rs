//! The kubectl reads the agent asks Groove for, checked before they run.

mod check;
mod flags;
mod run;

#[cfg(test)]
mod tests;

pub use check::{Checked, check};
pub use run::{Answer, LIMIT, run};
