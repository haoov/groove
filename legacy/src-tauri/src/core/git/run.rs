use std::process::Output;
use std::time::Duration;

/// A git call that runs longer than this is killed. Covers a hung network fetch.
const TIMEOUT: Duration = Duration::from_secs(120);

/// Replaces the credentials in any `scheme://user:secret@host` with `<redacted>`.
/// Git echoes the remote URL in its errors, credentials included.
pub(crate) fn redact(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find("://") {
        let (head, tail) = rest.split_at(at + 3);
        out.push_str(head);
        let end = tail
            .find(|c: char| c == '/' || c == '\'' || c == '"' || c == ')' || c.is_whitespace())
            .unwrap_or(tail.len());
        let (authority, after) = tail.split_at(end);
        match authority.rsplit_once('@') {
            Some((_, host)) => {
                out.push_str("<redacted>@");
                out.push_str(host);
            }
            None => out.push_str(authority),
        }
        rest = after;
    }
    out.push_str(rest);
    out
}

/// Run `git <args>` in `dir` and return its raw Output.
/// Every git spawn goes through here; call sites match on git's English messages.
pub async fn output(dir: &str, args: &[&str]) -> anyhow::Result<Output> {
    let dir = dir.to_string();
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let joined = redact(&args.join(" "));
    let detail = format!("git {joined}");
    crate::core::timing::timed("subprocess", detail, async move {
        let mut cmd = tokio::process::Command::new("git");
        cmd.args(&args)
            .env("LC_ALL", "C")
            .env("LANG", "C")
            // Never prompt: a missing credential must fail, not wait on /dev/tty for ever.
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_SSH_COMMAND", "ssh -oBatchMode=yes")
            .current_dir(&dir)
            .kill_on_drop(true);
        match tokio::time::timeout(TIMEOUT, cmd.output()).await {
            Ok(res) => res.map_err(|e| anyhow::anyhow!("failed to run git {joined}: {e}")),
            Err(_) => Err(anyhow::anyhow!(
                "git {joined} timed out after {}s",
                TIMEOUT.as_secs()
            )),
        }
    })
    .await
}

/// Run `git <args>` and return stdout; a non-zero exit becomes an error carrying stderr.
pub async fn run(dir: &str, args: &[&str]) -> anyhow::Result<String> {
    let joined = redact(&args.join(" "));
    let out = output(dir, args).await?;
    if !out.status.success() {
        return Err(anyhow::anyhow!(
            "git {joined} failed: {}",
            redact(String::from_utf8_lossy(&out.stderr).trim())
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

/// Whether `dir` is inside a git repository.
pub async fn is_repository(dir: &str) -> bool {
    output(dir, &["rev-parse", "--git-dir"])
        .await
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::redact;

    #[test]
    fn credentials_never_survive_redaction() {
        assert_eq!(
            redact(
                "fatal: unable to access 'https://oauth2:glpat-xyz@gitlab.example.com/g/p.git/'"
            ),
            "fatal: unable to access 'https://<redacted>@gitlab.example.com/g/p.git/'"
        );
        assert_eq!(
            redact("remote: https://token@github.com/o/r"),
            "remote: https://<redacted>@github.com/o/r"
        );
        // A URL with no credentials is untouched, and so is ordinary text.
        assert_eq!(
            redact("cloned https://github.com/o/r"),
            "cloned https://github.com/o/r"
        );
        assert_eq!(redact("nothing to commit"), "nothing to commit");
        assert_eq!(
            redact("ssh://git@host:2222/g/p"),
            "ssh://<redacted>@host:2222/g/p"
        );
    }

    #[test]
    fn no_git_spawn_outside_core_git() {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut offenders = vec![];
        visit(&src, &mut offenders);
        assert!(
            offenders.is_empty(),
            "Command::new(\"git\") outside core/git: {offenders:?}"
        );

        fn visit(dir: &std::path::Path, offenders: &mut Vec<String>) {
            for entry in std::fs::read_dir(dir).unwrap().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    visit(&path, offenders);
                } else if path.extension().is_some_and(|e| e == "rs")
                    && !path.to_string_lossy().contains("/core/git/")
                    && std::fs::read_to_string(&path)
                        .unwrap()
                        .contains("Command::new(\"git\")")
                {
                    offenders.push(path.display().to_string());
                }
            }
        }
    }
}
