//! The approval pipeline: a write op becomes a pending confirmation the user
//! decides on. `bridge` is the mechanics (post, resolve, unblock the waiting
//! agent); `ops` is the catalog — every op's name and executor side by side.

mod bridge;
pub mod ops;

// Glob re-export: tauri::generate_handler! looks up __cmd__* symbols at the
// same path as the command function.
pub use bridge::*;
