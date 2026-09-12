//! The approval pipeline. `bridge` posts and resolves confirmations; `ops`
//! catalogs every gated op with its executor.

mod bridge;
pub mod ops;

// Keep the glob re-export: tauri::generate_handler! resolves `__cmd__*` at the function's path.
pub use bridge::*;
