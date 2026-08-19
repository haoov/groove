use std::process::Output;

/// Run `git <args>` in `dir` off the async runtime and return its raw Output,
/// for callers that inspect status/stderr themselves. Every git invocation in
/// the app goes through here: never blocks a tokio worker, always speaks
/// English (call sites match on git's messages), always lands in the timing log.
pub async fn output(dir: &str, args: &[&str]) -> anyhow::Result<Output> {
    let dir = dir.to_string();
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let joined = args.join(" ");
    let detail = format!("git {joined}");
    crate::core::timing::timed("subprocess", detail, async move {
        tokio::task::spawn_blocking(move || {
            std::process::Command::new("git")
                .args(&args)
                .env("LC_ALL", "C")
                .env("LANG", "C")
                .current_dir(&dir)
                .output()
        })
        .await?
        .map_err(|e| anyhow::anyhow!("failed to run git {joined}: {e}"))
    })
    .await
}

/// Run `git <args>`, check the exit status, and return stdout. A non-zero exit
/// becomes an error carrying stderr.
pub async fn run(dir: &str, args: &[&str]) -> anyhow::Result<String> {
    let joined = args.join(" ");
    let out = output(dir, args).await?;
    if !out.status.success() {
        return Err(anyhow::anyhow!(
            "git {joined} failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

/// Whether `dir` is inside a git repository — the cheap validity check that
/// used to cost a libgit2 dependency.
pub async fn is_repository(dir: &str) -> bool {
    output(dir, &["rev-parse", "--git-dir"])
        .await
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    /// Every git spawn goes through this module — anywhere else loses the
    /// forced-English output, the blocking-thread hop, and the timing log.
    #[test]
    fn no_git_spawn_outside_core_git() {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut offenders = vec![];
        visit(&src, &mut offenders);
        assert!(offenders.is_empty(), "Command::new(\"git\") outside core/git: {offenders:?}");

        fn visit(dir: &std::path::Path, offenders: &mut Vec<String>) {
            for entry in std::fs::read_dir(dir).unwrap().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    visit(&path, offenders);
                } else if path.extension().is_some_and(|e| e == "rs")
                    && !path.to_string_lossy().contains("/core/git/")
                    && std::fs::read_to_string(&path).unwrap().contains("Command::new(\"git\")")
                {
                    offenders.push(path.display().to_string());
                }
            }
        }
    }
}
