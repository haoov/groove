//! One way out of the process: HTTP, on a single client.

mod auth;
mod client;
mod error;
mod graphql;
mod redact;
mod response;
mod rest;

#[cfg(test)]
mod tests;

pub use auth::TokenSource;
pub use client::{Client, Request};
pub use error::{Error, Result};
pub use graphql::Graphql;
pub use reqwest::{Method, StatusCode};
pub use response::Response;
pub use rest::Rest;
