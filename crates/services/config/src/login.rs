//! Setup's `claude auth login`, on a terminal of its own.

use std::path::Path;

use groove_terminal::{Hooks, Terminal};
use groove_types::{AnsiPalette, Error, ErrorKind};

/// `claude auth login` on a terminal of `cols` by `rows`.
pub async fn login(
    home: &Path,
    (cols, rows): (u16, u16),
    palette: AnsiPalette,
    hooks: Hooks,
) -> Result<Terminal, Error> {
    let spec = groove_agent_launch::login(home, cols, rows).await;
    Terminal::spawn(spec, palette, hooks)
        .await
        .map_err(|e| Error::new(ErrorKind::Agent, e.to_string()))
}

/// The `claude` the environment check and a launch run.
pub async fn claude_bin(home: &Path) -> String {
    groove_agent_launch::claude_bin(home).await
}
