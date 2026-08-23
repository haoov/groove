//! Where tasks come from. One module per source, behind a shared surface.

pub mod notion;
pub mod types;

// Glob re-export is required: tauri::generate_handler! looks up __cmd__* symbols
// at the same path as the function.
pub use notion::*;
