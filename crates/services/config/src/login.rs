//! A CLI's browser login, `claude` or `inc`, on a terminal of its own.

use std::path::Path;

use groove_terminal::{Hooks, Terminal};
use groove_types::{AnsiPalette, Error, ErrorKind};

/// The CLI a login runs.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum SignIn {
    /// `claude auth login`, from Setup.
    #[default]
    Claude,
    /// `inc auth login`, from Alerts.
    Inc,
}

/// That CLI's login on a terminal of `cols` by `rows`.
pub async fn login(
    home: &Path,
    (which, cols, rows): (SignIn, u16, u16),
    palette: AnsiPalette,
    hooks: Hooks,
) -> Result<Terminal, Error> {
    let spec = match which {
        SignIn::Claude => groove_agent_launch::login(home, cols, rows).await,
        SignIn::Inc => groove_agent_launch::inc_login(home, cols, rows),
    };
    Terminal::spawn(spec, palette, hooks)
        .await
        .map_err(|e| Error::new(ErrorKind::Agent, e.to_string()))
}

/// The `claude` the environment check and a launch run.
pub async fn claude_bin(home: &Path) -> String {
    groove_agent_launch::claude_bin(home).await
}
