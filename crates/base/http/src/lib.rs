//! One way out of the process: HTTP, on a single client.

mod auth;
mod client;
mod error;
mod redact;
mod response;

#[cfg(test)]
mod tests;

pub use client::{Client, Request};
pub use error::{Error, Result};
pub use reqwest::{Method, StatusCode};
pub use response::Response;
