//! One way out of the process: HTTP, on a single client.

mod auth;
mod client;
mod error;
mod redact;
mod response;

pub use auth::TokenSource;
pub use client::{Client, Request};
pub use error::{Error, Result};
pub use redact::redact_url;
pub use reqwest::{Method, StatusCode};
pub use response::Response;
