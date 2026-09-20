//! The CLI a host's token is asked of, and where in its output the token stands.

use std::sync::Mutex;

use groove_exec::run::Run;
use groove_http::{Error, Result, TokenSource};

/// The tool that holds a host's token.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cli {
    Gh,
    Glab,
}

/// The stream a tool prints its answer on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Stream {
    Out,
    Err,
}

impl Cli {
    pub fn program(self) -> &'static str {
        match self {
            Cli::Gh => "gh",
            Cli::Glab => "glab",
        }
    }

    /// The arguments that print one host's token.
    fn args(self, host: &str) -> Vec<String> {
        let host = host.to_string();
        match self {
            Cli::Gh => vec!["auth".into(), "token".into(), "--hostname".into(), host],
            Cli::Glab => vec![
                "auth".into(),
                "status".into(),
                "--hostname".into(),
                host,
                "--show-token".into(),
            ],
        }
    }

    /// `glab` writes its report to the error stream.
    fn stream(self) -> Stream {
        match self {
            Cli::Gh => Stream::Out,
            Cli::Glab => Stream::Err,
        }
    }

    /// The token in what the tool printed: `gh` prints it alone, `glab` in a report.
    pub(crate) fn read(self, printed: &str) -> Option<String> {
        let found = match self {
            Cli::Gh => printed.trim().to_string(),
            Cli::Glab => reported(printed)?,
        };
        (!found.is_empty()).then_some(found)
    }

    fn login(self, host: &str) -> String {
        format!("run `{} auth login --hostname {host}`", self.program())
    }
}

/// The token on the one line of `glab`'s report that names it.
fn reported(printed: &str) -> Option<String> {
    printed
        .lines()
        .filter(|line| line.contains("Token"))
        .filter_map(|line| line.rsplit_once(": "))
        .map(|(_, token)| token.trim().to_string())
        .find(|token| !token.is_empty())
}

/// One host's token, asked of its CLI once and held until the server refuses it.
pub struct CliToken {
    cli: Cli,
    host: String,
    held: Mutex<Option<String>>,
}

impl CliToken {
    pub fn new(cli: Cli, host: impl Into<String>) -> Self {
        Self {
            cli,
            host: host.into(),
            held: Mutex::new(None),
        }
    }

    /// Runs the tool and reads the token out of the stream it answers on.
    async fn asked(&self) -> Result<String> {
        let run = Run::new(self.cli.program()).args(self.cli.args(&self.host));
        let described = run.describe();
        let output = run
            .output()
            .await
            .map_err(|e| Error::Auth(format!("{described}: {e}")))?;
        let printed = match self.cli.stream() {
            Stream::Out => &output.stdout,
            Stream::Err => &output.stderr,
        };
        let printed = String::from_utf8_lossy(printed);
        match output.status.success() {
            true => self.found(&printed),
            false => Err(self.refused()),
        }
    }

    fn found(&self, printed: &str) -> Result<String> {
        self.cli.read(printed).ok_or_else(|| self.refused())
    }

    fn refused(&self) -> Error {
        Error::Auth(format!(
            "no token for {}: {}",
            self.host,
            self.cli.login(&self.host)
        ))
    }
}

impl TokenSource for CliToken {
    async fn token(&self) -> Result<String> {
        let held = self.held.lock().ok().and_then(|held| held.clone());
        if let Some(token) = held {
            return Ok(token);
        }
        let token = self.asked().await?;
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
