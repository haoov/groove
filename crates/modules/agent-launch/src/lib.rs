//! One launch of the agent: `claude` with its session, its prompt, its skills and,
//! when the loopback server runs, its MCP config and hooks.

mod error;
mod files;
mod loopback;
mod prompt;
mod session;

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use crate::prompt::core_prompt;
pub use error::{Error, Result};
use groove_exec::pty::PtySpec;
use groove_types::Session;
pub use loopback::{Loopback, Tools};
pub use session::hand_over;

/// Where a launch reads and writes.
pub struct Paths<'a> {
    /// `$HOME`, where Claude keeps its sessions.
    pub home: &'a Path,
    /// Where the launch files go, private to the user.
    pub launch_dir: &'a Path,
    /// The agent's working directory.
    pub cwd: &'a Path,
}

/// The command line, ready to spawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Launch {
    pub program: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub cwd: PathBuf,
}

impl Launch {
    pub fn plan(
        session: &Session,
        paths: &Paths<'_>,
        plugin_dirs: &[PathBuf],
        loopback: Option<&Loopback>,
    ) -> Result<Self> {
        let files = files::LaunchDir::new(paths.launch_dir, session.id.as_str());
        let mut args = session::identity_args(session, paths);
        if let Some(loopback) = loopback {
            args.extend(loopback.args(&files)?);
        }
        args.push("--append-system-prompt-file".into());
        args.push(files.write("prompt.md", &core_prompt(session))?);
        for dir in plugin_dirs {
            args.push("--plugin-dir".into());
            args.push(dir.to_string_lossy().into_owned());
        }
        Ok(Self {
            program: claude_bin(paths.home),
            args,
            env: env(),
            cwd: paths.cwd.to_path_buf(),
        })
    }

    pub fn spec(self, cols: u16, rows: u16) -> PtySpec {
        PtySpec {
            program: self.program,
            args: self.args,
            cwd: self.cwd,
            env: self.env,
            rows,
            cols,
        }
    }
}

/// `~/.local/bin/claude`, then the system paths, then whatever `PATH` finds.
fn claude_bin(home: &Path) -> String {
    let local = home.join(".local/bin/claude");
    if local.is_file() {
        return local.to_string_lossy().into_owned();
    }
    ["/usr/local/bin/claude", "/usr/bin/claude"]
        .into_iter()
        .find(|p| Path::new(p).is_file())
        .unwrap_or("claude")
        .to_string()
}

/// No MCP idle timeout and a 24h cap: a gated write may wait on a human for hours.
fn env() -> Vec<(String, String)> {
    vec![
        ("CLAUDE_CODE_MCP_TOOL_IDLE_TIMEOUT".into(), "0".into()),
        ("MCP_TOOL_TIMEOUT".into(), "86400000".into()),
    ]
}
