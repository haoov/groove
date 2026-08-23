//! Everything that speaks the Notion API.
//!
//! `api` holds the HTTP verbs and is `pub(super)`: the compiler makes this the
//! only module that can reach Notion. Everyone else calls named functions —
//! `tasks::set_status`, `body::replace_from_markdown` — never a `v1/...` path.

mod api;
pub mod body;
pub mod create;
pub mod detect;
pub mod hours;
pub mod markdown;
pub mod page;
pub mod properties;
mod provider;
pub mod schema;
pub mod tasks;
pub mod users;

// Glob re-exports: tauri::generate_handler! looks up __cmd__* symbols at the
// same path as the command function, and `pub use` of the fn alone misses them.
pub use body::*;
pub use create::*;
pub use properties::*;
pub use schema::*;
pub use tasks::*;
pub use users::*;

pub use provider::NotionProvider;
