//! The bearer token a host is called with, from its CLI or handed over; never stored.

mod cli;

#[cfg(test)]
mod tests;

pub use cli::{Cli, CliToken};

use groove_http::{Result, TokenSource};

/// Where a call's token comes from.
pub enum Token {
    /// Asked of the host's CLI and kept until the server refuses it.
    Cli(CliToken),
    /// Handed over, for a host whose CLI is not logged in.
    Fixed(String),
}

impl Token {
    /// The token `gh` holds for a host.
    pub fn gh(host: impl Into<String>) -> Self {
        Token::Cli(CliToken::new(Cli::Gh, host))
    }

    /// The token `glab` holds for a host.
    pub fn glab(host: impl Into<String>) -> Self {
        Token::Cli(CliToken::new(Cli::Glab, host))
    }

    /// A configured token where there is one, else the CLI's.
    pub fn configured(configured: Option<String>, cli: Cli, host: &str) -> Self {
        match configured {
            Some(token) => Token::Fixed(token),
            None => Token::Cli(CliToken::new(cli, host)),
        }
    }
}

impl TokenSource for Token {
    async fn token(&self) -> Result<String> {
        match self {
            Token::Cli(cli) => cli.token().await,
            Token::Fixed(token) => Ok(token.clone()),
        }
    }

    fn invalidate(&self) {
        match self {
            Token::Cli(cli) => cli.invalidate(),
            Token::Fixed(_) => {}
        }
    }
}
