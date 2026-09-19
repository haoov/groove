//! The token a host is called with: what `gh` holds, or one handed over.

use std::sync::Mutex;

use groove_exec::run::Run;
use groove_http::{Result as HttpResult, TokenSource};

use crate::Error;

/// Where a call's bearer token comes from.
pub enum Token {
    /// Asked of `gh` and kept until the server refuses it.
    Gh(GhToken),
    /// Handed over, for a host that has no `gh`.
    Fixed(String),
}

impl TokenSource for Token {
    async fn token(&self) -> HttpResult<String> {
        match self {
            Token::Gh(gh) => gh.token().await,
            Token::Fixed(token) => Ok(token.clone()),
        }
    }

    fn invalidate(&self) {
        match self {
            Token::Gh(gh) => gh.invalidate(),
            Token::Fixed(_) => {}
        }
    }
}

pub struct GhToken {
    host: String,
    held: Mutex<Option<String>>,
}

impl GhToken {
    pub fn new(host: impl Into<String>) -> Self {
        Self {
            host: host.into(),
            held: Mutex::new(None),
        }
    }

    async fn asked(&self) -> Result<String, Error> {
        let token = Run::new("gh")
            .args(["auth", "token", "--hostname", &self.host])
            .text()
            .await
            .map_err(|source| Error::Token {
                host: self.host.clone(),
                source,
            })?;
        let token = token.trim().to_string();
        match token.is_empty() {
            true => Err(Error::Invalid(format!(
                "`gh` has no token for {}: run `gh auth login --hostname {}`",
                self.host, self.host
            ))),
            false => Ok(token),
        }
    }
}

impl TokenSource for GhToken {
    async fn token(&self) -> HttpResult<String> {
        let held = self.held.lock().ok().and_then(|held| held.clone());
        if let Some(token) = held {
            return Ok(token);
        }
        let token = self
            .asked()
            .await
            .map_err(|e| groove_http::Error::Auth(e.to_string()))?;
        if let Ok(mut held) = self.held.lock() {
            *held = Some(token.clone());
        }
        Ok(token)
    }

    fn invalidate(&self) {
        if let Ok(mut held) = self.held.lock() {
            *held = None;
        }
    }
}
