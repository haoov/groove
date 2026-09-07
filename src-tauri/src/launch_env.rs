//! Widen PATH so a desktop launch finds the tools a shell profile adds.
//! A `.desktop` launch inherits `/usr/local/bin:/usr/bin:/bin` only; `glab`, `gh` and `claude` live elsewhere.

/// Directories a desktop launch does not have on PATH. Appended, never prepended.
fn extra_dirs() -> Vec<std::path::PathBuf> {
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from);
    let mut dirs: Vec<std::path::PathBuf> = vec![
        "/usr/local/bin".into(),
        "/home/linuxbrew/.linuxbrew/bin".into(),
        "/opt/homebrew/bin".into(),
        "/snap/bin".into(),
    ];
    if let Some(home) = home {
        for rel in [
            ".local/bin",
            ".cargo/bin",
            ".linuxbrew/bin",
            ".npm-global/bin",
            "bin",
        ] {
            dirs.push(home.join(rel));
        }
    }
    dirs
}

/// `current` plus `extra_dirs()`; pure.
fn widened(current: &std::ffi::OsStr) -> Vec<std::path::PathBuf> {
    let mut paths: Vec<std::path::PathBuf> = std::env::split_paths(current).collect();
    for dir in extra_dirs() {
        if dir.is_dir() && !paths.contains(&dir) {
            paths.push(dir);
        }
    }
    paths
}

/// Extend this process's PATH with `extra_dirs()`. Call once, before anything spawns a child.
pub fn widen_path() {
    let current = std::env::var_os("PATH").unwrap_or_default();
    if let Ok(joined) = std::env::join_paths(widened(&current)) {
        std::env::set_var("PATH", &joined);
        tracing::debug!("PATH widened to {}", joined.to_string_lossy());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn given(entries: &[&str]) -> std::ffi::OsString {
        std::env::join_paths(entries).expect("joinable")
    }

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

    #[test]
    fn only_adds_directories_that_exist() {
        let current = given(&["/usr/bin"]);
        let before: Vec<PathBuf> = std::env::split_paths(&current).collect();
        for dir in widened(&current) {
            if !before.contains(&dir) {
                assert!(
                    dir.is_dir(),
                    "{} was added but does not exist",
                    dir.display()
                );
            }
        }
    }
}
