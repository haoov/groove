/// Expand a leading `~` to `$HOME`; a child process spawned without a shell never expands `~`.
pub fn expand_tilde(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        let home = std::env::var("HOME").unwrap_or_default();
        format!("{home}/{rest}")
    } else if path == "~" {
        std::env::var("HOME").unwrap_or_default()
    } else {
        path.to_string()
    }
}

/// Join a worktree-relative path; rejects absolute paths and `..` segments.
pub fn safe_join(worktree_path: &str, rel: &str) -> Result<std::path::PathBuf, String> {
    let rel = rel.trim().trim_start_matches('/');
    if rel.is_empty() {
        return Err("empty path".to_string());
    }
    let mut p = std::path::PathBuf::from(worktree_path);
    for comp in std::path::Path::new(rel).components() {
        match comp {
            std::path::Component::Normal(s) => p.push(s),
            std::path::Component::CurDir => {}
            _ => return Err(format!("invalid path: {rel}")),
        }
    }
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn safe_join_keeps_a_path_under_the_root() {
        assert_eq!(
            safe_join("/w", "src/a.rs").unwrap(),
            PathBuf::from("/w/src/a.rs")
        );
        assert_eq!(
            safe_join("/w", "./src/a.rs").unwrap(),
            PathBuf::from("/w/src/a.rs")
        );
        // A leading slash is stripped, never honoured as an absolute path.
        assert_eq!(
            safe_join("/w", "/etc/passwd").unwrap(),
            PathBuf::from("/w/etc/passwd")
        );
    }

    #[test]
    fn safe_join_rejects_an_escape_and_an_empty_path() {
        assert!(safe_join("/w", "../etc/passwd").is_err());
        assert!(safe_join("/w", "src/../../etc/passwd").is_err());
        assert!(safe_join("/w", "").is_err());
        assert!(safe_join("/w", "   ").is_err());
    }

    #[test]
    fn tilde_expands_and_plain_paths_pass_through() {
        let home = std::env::var("HOME").unwrap();
        assert_eq!(expand_tilde("~"), home);
        assert_eq!(expand_tilde("~/x/y"), format!("{home}/x/y"));
        assert_eq!(expand_tilde("/abs/path"), "/abs/path");
        assert_eq!(expand_tilde("rel/~x"), "rel/~x");
    }
}
