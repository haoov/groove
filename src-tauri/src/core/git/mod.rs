//! Git plumbing: the spawner, the URL parser and the cached ref answers.

pub mod cache;
pub mod porcelain;
pub mod refs;
pub mod run;
mod url;

pub use run::{output, run};
pub use url::{parse_git_url, url_host};
