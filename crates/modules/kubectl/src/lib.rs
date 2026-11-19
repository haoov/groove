//! The kubectl reads the agent asks for, checked against the session's clusters then run bounded, and the commands it runs in a pod.

mod check;
mod exec;
mod flags;
mod run;
mod secrets;

#[cfg(test)]
mod tests;

pub use check::{Checked, check};
pub use exec::{Exec, LONGEST, RUNS, exec};
pub use run::{Answer, LIMIT, run};
