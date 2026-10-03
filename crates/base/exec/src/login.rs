//! The `PATH` the user's login shell sets, which a desktop launch does not inherit.

use std::sync::OnceLock;
use std::time::Duration;

use crate::run::Run;

const WAIT: Duration = Duration::from_secs(5);
const MARK: &str = "__GROOVE_PATH__";

static ADOPTED: OnceLock<String> = OnceLock::new();

/// Gives every child spawned from now on `path` as its `PATH`. The first call wins.
pub fn adopt(path: String) {
    let _ = ADOPTED.set(path);
}

pub(crate) fn adopted() -> Option<&'static str> {
    ADOPTED.get().map(String::as_str)
}

/// The `PATH` an interactive login `shell` exports, or `None` when it fails or hangs.
pub async fn path(shell: &str) -> Option<String> {
    let print = format!("printf '%s%s%s' {MARK} \"$PATH\" {MARK}");
    let run = Run::new(shell)
        .args(["-l", "-i", "-c", &print])
        .timeout(WAIT);
    marked(&run.text().await.ok()?)
}

/// The text between the two marks, apart from what the startup files print.
pub(crate) fn marked(out: &str) -> Option<String> {
    let (_, rest) = out.split_once(MARK)?;
    let (path, _) = rest.split_once(MARK)?;
    (!path.is_empty()).then(|| path.to_string())
}
