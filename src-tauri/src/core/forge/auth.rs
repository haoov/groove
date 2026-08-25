//! Forge tokens, borrowed from the CLIs at runtime.
//!
//! `glab` and `gh` own the login flow, the keyring, and the per-host identity —
//! the one thing they are irreplaceable for. Everything else talks to the APIs
//! directly (see `api`), so the CLIs are consulted exactly once per host for a
//! token that lives in memory and is never persisted by us.

use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};

/// A forge CLI is not installed.
///
/// Typed rather than a message, because the caller has to TELL IT APART from a
/// failure: a machine with no `glab` is a machine with no GitLab repos, and the
/// review queue must stay quiet about it instead of showing an error.
#[derive(Debug)]
pub(crate) struct CliMissing(pub &'static str);

impl std::fmt::Display for CliMissing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "`{}` is not installed, or not on the PATH this app was launched with",
            self.0
        )
    }
}
impl std::error::Error for CliMissing {}

/// True when the failure is only that the CLI is absent.
pub(crate) fn is_cli_missing(e: &anyhow::Error) -> bool {
    e.downcast_ref::<CliMissing>().is_some()
}

fn cache() -> &'static Mutex<HashMap<String, String>> {
    static TOKENS: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    TOKENS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// The token for a host, from cache or from its CLI.
pub(crate) async fn token(platform: super::Platform, host: &str) -> anyhow::Result<String> {
    if let Ok(map) = cache().lock() {
        if let Some(t) = map.get(host) {
            return Ok(t.clone());
        }
    }
    let token = fetch_token(platform, host).await?;
    if let Ok(mut map) = cache().lock() {
        map.insert(host.to_string(), token.clone());
    }
    Ok(token)
}

/// Drop a host's cached token — called on a 401 so the next request re-asks the
/// CLI (the user may have re-logged in, or the token rotated).
pub(crate) fn forget(host: &str) {
    if let Ok(mut map) = cache().lock() {
        map.remove(host);
    }
}

async fn fetch_token(platform: super::Platform, host: &str) -> anyhow::Result<String> {
    match platform {
        super::Platform::Github => {
            let out = run_cli("gh", &["auth", "token", "--hostname", host]).await?;
            let token = out.stdout.trim().to_string();
            if token.is_empty() {
                return Err(anyhow::anyhow!(
                    "`gh` has no token for {host} — run `gh auth login --hostname {host}`"
                ));
            }
            Ok(token)
        }
        super::Platform::Gitlab => {
            // `glab auth status -t` prints every configured host with its token.
            // Historically on stderr, sometimes stdout — parse both.
            let out = run_cli("glab", &["auth", "status", "-t"]).await?;
            let combined = format!("{}\n{}", out.stdout, out.stderr);
            glab_token_for(&combined, host).ok_or_else(|| {
                anyhow::anyhow!(
                    "`glab` has no token for {host} — run `glab auth login --hostname {host}`"
                )
            })
        }
    }
}

struct CliOutput {
    stdout: String,
    stderr: String,
}

async fn run_cli(bin: &'static str, args: &[&str]) -> anyhow::Result<CliOutput> {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let printable = format!("{bin} {}", args.join(" "));
    let out = crate::core::timing::timed("subprocess", printable.clone(), async {
        tokio::task::spawn_blocking(move || {
            std::process::Command::new(bin).args(&args).output()
        })
        .await
    })
    .await?;

    match out {
        Ok(o) => Ok(CliOutput {
            stdout: String::from_utf8_lossy(&o.stdout).to_string(),
            stderr: String::from_utf8_lossy(&o.stderr).to_string(),
        }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Err(anyhow::Error::new(CliMissing(bin)))
        }
        Err(e) => Err(anyhow::anyhow!("failed to run {printable}: {e}")),
    }
}

/// Pull `host`'s token out of `glab auth status -t` output. The output lists
/// hosts as headings with indented "✓ …" detail lines; the token line reads
/// "Token found: <t>" (glab ≥1.60) or "Token: <t>" (older). Match either by
/// keying on a line that mentions "Token" and taking the last ": " segment.
fn glab_token_for(output: &str, host: &str) -> Option<String> {
    let mut in_host = false;
    for line in output.lines() {
        // A heading is a non-indented, non-empty line — the bare host name.
        let is_detail = line.starts_with(char::is_whitespace);
        let trimmed = line.trim();
        if !is_detail && !trimmed.is_empty() {
            in_host = trimmed == host;
            continue;
        }
        if !in_host || !trimmed.contains("Token") {
            continue;
        }
        // "✓ Token found: glpat-…" → the part after the final ": ".
        if let Some((_, token)) = trimmed.rsplit_once(": ") {
            let token = token.trim();
            if !token.is_empty() && !token.contains('*') {
                return Some(token.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::glab_token_for;

    /// The real shape of `glab auth status -t` on glab 1.6x: hosts as headings,
    /// indented "✓ …" details, and "Token found:" (not "Token:").
    const STATUS: &str = "\
gitlab.com
  ✓ Logged in to gitlab.com as haoov (/home/r/.config/glab-cli/config.yml)
  ✓ API calls for gitlab.com are made over https protocol.
  ✓ REST API Endpoint: https://gitlab.com/api/v4/
  ✓ Token found: glpat-FIRSTtoken.01.abc
gitlab.wiremind.io
  ✓ Logged in to gitlab.wiremind.io as rsabbah (/home/r/.config/glab-cli/config.yml)
  ✓ Token found: glpat-SECONDtoken.01.def
";

    #[test]
    fn finds_the_token_of_the_asked_host_only() {
        assert_eq!(
            glab_token_for(STATUS, "gitlab.com").as_deref(),
            Some("glpat-FIRSTtoken.01.abc")
        );
        assert_eq!(
            glab_token_for(STATUS, "gitlab.wiremind.io").as_deref(),
            Some("glpat-SECONDtoken.01.def")
        );
        assert_eq!(glab_token_for(STATUS, "gitlab.example.org"), None);
    }

    /// The older "Token:" wording must still parse — the "REST API Endpoint:"
    /// line, which also contains a colon, must NOT be mistaken for it.
    #[test]
    fn accepts_the_older_wording_and_ignores_other_colon_lines() {
        let old = "gitlab.com\n  ✓ REST API Endpoint: https://gitlab.com/api/v4/\n  ✓ Token: glpat-OLD\n";
        assert_eq!(glab_token_for(old, "gitlab.com").as_deref(), Some("glpat-OLD"));
    }

    /// Without `-t`, glab masks the token — a masked value must not be used.
    #[test]
    fn a_masked_token_is_not_a_token() {
        let masked = "gitlab.com\n  ✓ Token found: **************\n";
        assert_eq!(glab_token_for(masked, "gitlab.com"), None);
    }
}
