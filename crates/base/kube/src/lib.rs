//! One way out of the process: the Kubernetes API, one client per kubeconfig context.

mod client;
mod discovery;
mod error;
mod kubeconfig;
mod objects;
mod table;
mod watch;

#[cfg(test)]
mod tests;

pub use client::Client;
pub use discovery::Kind;
pub use error::{Error, Result};
pub use kubeconfig::{Auth, Context, contexts};
pub use objects::{Listed, Narrowed, Seen};
pub use table::{Column, Page, Query, Row};
pub use watch::Change;
