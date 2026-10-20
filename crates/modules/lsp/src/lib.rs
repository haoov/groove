//! A language server over stdio: its documents, diagnostics, definitions, hover and completions.
//! Columns are characters here, whatever unit the server counts in.

mod client;
mod completion;
mod documents;
mod frame;
mod incoming;
mod link;
mod position;
mod protocol;
mod uri;
mod watched;

#[cfg(test)]
mod tests;

pub use client::{Client, Launch, connect, start};
pub use incoming::Hooks;
pub use position::Encoding;
