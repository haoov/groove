//! Making a desktop launch look like a terminal launch.
//!
//! Started from a `.desktop` entry, the app inherits the session's minimal PATH —
//! typically `/usr/local/bin:/usr/bin:/bin`. It does NOT inherit what a shell
//! profile adds, and that is exactly where the tools this app shells out to live:
//! `glab` and `gh` from linuxbrew, `claude` from an npm global, anything from
//! `~/.cargo/bin`.
//!
//! The symptom is misleading: `glab` reports "No such file or directory" while
//! `which glab` in a terminal finds it, so the app looks broken rather than
//! mis-launched. Widening PATH once at startup fixes every caller at once —
//! including the agent and terminal PTYs, which inherit this process's environment.

/// Directories a developer's tools land in but a desktop launch does not know about.
/// Appended, never prepended: a PATH the user did set stays authoritative.
fn extra_dirs() -> Vec<std::path::PathBuf> {
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from);
    let mut dirs: Vec<std::path::PathBuf> = vec![
        "/usr/local/bin".into(),
        "/home/linuxbrew/.linuxbrew/bin".into(),
        "/opt/homebrew/bin".into(),
        "/snap/bin".into(),
    ];
    if let Some(home) = home {
        for rel in [".local/bin", ".cargo/bin", ".linuxbrew/bin", ".npm-global/bin", "bin"] {
            dirs.push(home.join(rel));
        }
    }
    dirs
}

/// `current` plus the directories above. Pure, so the rules can be checked without
/// the process environment — which the tests share, and which already holds entries
/// nobody here added.
fn widened(current: &std::ffi::OsStr) -> Vec<std::path::PathBuf> {
    let mut paths: Vec<std::path::PathBuf> = std::env::split_paths(current).collect();
    for dir in extra_dirs() {
        // Only real directories, and never a second copy of one already there.
        if dir.is_dir() && !paths.contains(&dir) {
            paths.push(dir);
        }
    }
    paths
}

/// Extend this process's PATH with the directories above, keeping order and
/// dropping duplicates. Call once, before anything spawns a child.
pub fn widen_path() {
    let current = std::env::var_os("PATH").unwrap_or_default();
    if let Ok(joined) = std::env::join_paths(&widened(&current)) {
        std::env::set_var("PATH", &joined);
        tracing::debug!("PATH widened to {}", joined.to_string_lossy());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// A PATH of our own, so a test never depends on the runner's.
    fn given(entries: &[&str]) -> std::ffi::OsString {
        std::env::join_paths(entries).expect("joinable")
    }

    /// The user's own PATH must keep priority: a tool they put first stays first.
    #[test]
    fn appends_without_reordering_what_was_there() {
        let current = given(&["/usr/bin", "/bin"]);
        let before: Vec<PathBuf> = std::env::split_paths(&current).collect();
        assert!(
            widened(&current).starts_with(&before),
            "existing entries must keep their order"
        );
    }

    #[test]
    fn is_idempotent() {
        let once = widened(&given(&["/usr/bin"]));
        let twice = widened(&std::env::join_paths(&once).unwrap());
        assert_eq!(once, twice, "no duplicate entries");
    }

    /// Only what we ADD has to exist. The environment already carries entries that
    /// do not — `~/.local/bin` on a CI runner — and those are not ours to judge.
    #[test]
    fn only_adds_directories_that_exist() {
        let current = given(&["/usr/bin"]);
        let before: Vec<PathBuf> = std::env::split_paths(&current).collect();
        for dir in widened(&current) {
            if !before.contains(&dir) {
                assert!(dir.is_dir(), "{} was added but does not exist", dir.display());
            }
        }
    }
}
