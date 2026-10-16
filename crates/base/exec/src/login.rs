//! The `PATH` the user's login shell sets, which a desktop launch does not inherit.

use std::sync::OnceLock;
use std::time::Duration;

use crate::run::Run;

const WAIT: Duration = Duration::from_secs(5);
const MARK: &str = "__GROOVE_PATH__";

/// Where Homebrew installs, on Apple silicon then on Intel.
#[cfg(target_os = "macos")]
const INSTALLS: &[&str] = &["/opt/homebrew/bin", "/usr/local/bin"];
#[cfg(not(target_os = "macos"))]
const INSTALLS: &[&str] = &[];

static ADOPTED: OnceLock<String> = OnceLock::new();

/// Gives every child spawned from now on `path` as its `PATH`. The first call wins.
pub fn adopt(path: String) {
    let _ = ADOPTED.set(path);
}

pub(crate) fn adopted() -> Option<&'static str> {
    ADOPTED.get().map(String::as_str)
}

/// The `PATH` an interactive login `shell` exports, else a non-interactive one's.
pub async fn path(shell: &str) -> Option<String> {
    let (interactive, plain) = tokio::join!(
        exported(shell, &["-l", "-i", "-c"]),
        exported(shell, &["-l", "-c"]),
    );
    interactive.or(plain)
}

/// `path` with the Homebrew directories it lacks appended.
pub fn with_installs(path: &str) -> String {
    joined(path, INSTALLS)
}

async fn exported(shell: &str, flags: &[&str]) -> Option<String> {
    let print = format!("printf '%s%s%s' {MARK} \"$PATH\" {MARK}");
    let run = Run::new(shell)
        .args(flags.iter().copied().chain([print.as_str()]))
        .timeout(WAIT);
    marked(&run.text().await.ok()?)
}

pub(crate) fn joined(path: &str, extra: &[&str]) -> String {
    let mut dirs: Vec<&str> = path.split(':').filter(|dir| !dir.is_empty()).collect();
    for dir in extra {
        if !dirs.contains(dir) {
            dirs.push(dir);
        }
    }
    dirs.join(":")
}

/// The text between the two marks, apart from what the startup files print.
pub(crate) fn marked(out: &str) -> Option<String> {
    let (_, rest) = out.split_once(MARK)?;
    let (path, _) = rest.split_once(MARK)?;
    (!path.is_empty()).then(|| path.to_string())
}
