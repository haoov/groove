//! Git's text into values. Pure functions, tested on captured output.

mod numstat;
mod porcelain;
mod quote;
mod url;

pub use numstat::{Counts, numstat};
pub use porcelain::{Change, porcelain};
pub use quote::unquote_path;
pub use url::RemoteUrl;
