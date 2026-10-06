//! One way out of the process: the Kubernetes API, one client per kubeconfig context.

mod client;
mod error;
mod kubeconfig;

#[cfg(test)]
mod tests;

pub use client::Client;
pub use error::{Error, Result};
pub use kubeconfig::{Auth, Context, contexts};
