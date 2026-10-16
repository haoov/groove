//! A language server over stdio: the documents Groove opens on it, the diagnostics it
//! publishes, and the definitions and hover it answers. Columns are characters on this
//! side of the module, whatever unit the server counts in.

mod client;
mod documents;
mod frame;
mod incoming;
mod link;
mod position;
mod protocol;
mod uri;

#[cfg(test)]
mod tests;

pub use client::{Client, Launch, connect, start};
pub use incoming::Hooks;
pub use position::Encoding;
