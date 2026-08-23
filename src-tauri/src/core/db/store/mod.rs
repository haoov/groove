//! The only module that speaks SQL. Every function takes an executor, so it
//! runs on the pool or inside a transaction unchanged.

pub mod annotations;
pub mod confirmations;
pub mod home;
pub mod mrs;
pub mod provider_tasks;
pub mod repos;
pub mod sessions;
pub mod time;
pub mod worktrees;

#[cfg(test)]
mod tests;
