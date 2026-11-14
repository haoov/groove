//! One launch of `claude`: its session, prompt and skills, and the loopback's MCP config and hooks.

mod files;
mod loopback;
mod prompt;
mod session;

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use crate::prompt::core_prompt;
use groove_exec::pty::PtySpec;
use groove_types::{Result, Session};
pub use loopback::{Loopback, Tools};
pub use session::{forget, fresh, hand_over};

/// Where a launch reads and writes.
pub struct Paths<'a> {
    /// `$HOME`, where Claude keeps its sessions.
    pub home: &'a Path,
    /// Where the launch files go, private to the user.
    pub launch_dir: &'a Path,
    /// The agent's working directory.
    pub cwd: &'a Path,
    /// The team's shared facts, when the shared repo holds them.
    pub knowledge: Option<&'a Path>,
}

/// The cluster tools the agent never runs itself: it reads through Groove's `kube` tool.
const DENIED: [&str; 5] = [
    "Bash(kubectl:*)",
    "Bash(helm:*)",
    "Bash(k9s:*)",
    "Bash(kubectx:*)",
    "Bash(kubens:*)",
];

/// The command line, ready to spawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Launch {
    pub program: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub cwd: PathBuf,
}

impl Launch {
    pub async fn plan(
        session: &Session,
        paths: &Paths<'_>,
        plugin_dirs: &[PathBuf],
        loopback: Option<&Loopback>,
    ) -> Result<Self> {
        let files = files::LaunchDir::new(paths.launch_dir, session.id.as_str());
        let mut args = session::identity_args(session, paths).await;
        if let Some(loopback) = loopback {
            args.extend(loopback.args(&files).await?);
        }
        args.push("--disallowedTools".into());
        args.extend(DENIED.iter().map(|one| one.to_string()));
        args.push("--append-system-prompt-file".into());
        let prompt = core_prompt(session, paths.knowledge);
        args.push(files.write("prompt.md", &prompt).await?);
        for dir in plugin_dirs {
            args.push("--plugin-dir".into());
            args.push(dir.to_string_lossy().into_owned());
        }
        Ok(Self {
            program: claude_bin(paths.home).await,
            args,
            env: env(),
            cwd: paths.cwd.to_path_buf(),
        })
    }

    /// The same, with `prompt` submitted as the agent's first message, ahead of every option.
    pub fn asking(mut self, prompt: &str) -> Self {
        self.args.insert(0, prompt.to_string());
        self
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

/// `claude auth login` in `home`, for a terminal the user signs in on.
pub async fn login(home: &Path, cols: u16, rows: u16) -> PtySpec {
    PtySpec {
        program: claude_bin(home).await,
        args: vec!["auth".into(), "login".into()],
        cwd: home.to_path_buf(),
        env: env(),
        rows,
        cols,
    }
}

/// `~/.local/bin/claude`, then the system paths, then whatever `PATH` finds.
pub async fn claude_bin(home: &Path) -> String {
    let local = home.join(".local/bin/claude");
    let system = ["/usr/local/bin/claude", "/usr/bin/claude"].map(PathBuf::from);
    for one in std::iter::once(local).chain(system) {
        if is_file(&one).await {
            return one.to_string_lossy().into_owned();
        }
    }
    "claude".to_string()
}

pub(crate) async fn is_file(path: &Path) -> bool {
    tokio::fs::metadata(path)
        .await
        .is_ok_and(|one| one.is_file())
}

/// No MCP idle timeout, and a 24h cap on a tool call.
fn env() -> Vec<(String, String)> {
    vec![
        ("CLAUDE_CODE_MCP_TOOL_IDLE_TIMEOUT".into(), "0".into()),
        ("MCP_TOOL_TIMEOUT".into(), "86400000".into()),
    ]
}
