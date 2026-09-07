//! Everything that speaks the Notion API.
//! `api` stays private to this module: callers outside `notion/` use named functions.

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
pub mod setup;
pub mod tasks;
pub mod users;

pub use create::*;

pub use provider::NotionProvider;
