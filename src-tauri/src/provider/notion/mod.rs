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

// The provider-GENERIC commands (body/property writes) moved to provider::write;
// find_notion_user is re-exported at provider:: directly from users. Only NewTask
// still flows up from create for the provider impl.
pub use create::*;

pub use provider::NotionProvider;
