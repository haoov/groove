//! Git plumbing: the one spawner, the one URL parser, the one answer to every
//! ref question — with a cache sized to how often the answers actually change.

pub mod cache;
pub mod refs;
pub mod run;
mod url;

pub use run::{output, run};
pub use url::parse_git_url;
