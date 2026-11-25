//! The token `argocd login --sso` keeps, refreshed by `argocd account get-user-info` once refused.

use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use groove_exec::run::Run;
use groove_http::{Error, Result};
use tokio::sync::Mutex;

use crate::Cli;

const WAIT: Duration = Duration::from_secs(20);

/// One refresh at a time: the refresh token rotates, and two refreshes at once would race on it.
static REFRESHING: Mutex<()> = Mutex::const_new(());

/// The CLI's server and token; a token already ended is refreshed first.
pub async fn argocd(home: &Path) -> Result<(String, groove_argocd::Token)> {
    let read = read(home).await?;
    match ended(&read.1) {
        true => argocd_refreshed(home, &read.1).await,
        false => Ok(read),
    }
}

/// A token other than `used`, refreshed by the CLI unless another call already did.
pub async fn argocd_refreshed(
    home: &Path,
    used: &groove_argocd::Token,
) -> Result<(String, groove_argocd::Token)> {
    refreshed(home, used, Cli::Argocd.program()).await
}

pub(crate) async fn refreshed(
    home: &Path,
    used: &groove_argocd::Token,
    program: &str,
) -> Result<(String, groove_argocd::Token)> {
    let _one = REFRESHING.lock().await;
    let read = self::read(home).await?;
    if &read.1 != used {
        return Ok(read);
    }
    let asked = Run::new(program).args(groove_argocd::REFRESH);
    let said = said(asked.timeout(WAIT).output().await);
    let read = self::read(home).await?;
    match &read.1 != used {
        true => Ok(read),
        false => Err(Error::Auth(format!(
            "the argocd login has ended{said}: {}",
            Cli::Argocd.login(&read.0)
        ))),
    }
}

/// The first line the refresh printed on failing, as ` (argocd: …)`; empty when it succeeded.
fn said(ran: groove_exec::Result<std::process::Output>) -> String {
    let first = match ran {
        Ok(out) if out.status.success() => return String::new(),
        Ok(out) => String::from_utf8_lossy(&out.stderr)
            .lines()
            .next()
            .map(str::to_string),
        Err(e) => Some(e.to_string()),
    };
    first.map_or_else(String::new, |line| format!(" (argocd: {line})"))
}

async fn read(home: &Path) -> Result<(String, groove_argocd::Token)> {
    let path = groove_argocd::config_path(home);
    let config = tokio::fs::read_to_string(&path).await.unwrap_or_default();
    groove_argocd::token_of(&config).ok_or_else(|| {
        Error::Auth(format!(
            "no Argo CD login: {}",
            Cli::Argocd.login("<server>")
        ))
    })
}

/// Past its end by its own `exp`; a token telling no end never is.
fn ended(token: &groove_argocd::Token) -> bool {
    let now = SystemTime::now().duration_since(UNIX_EPOCH);
    let now = now.map_or(0, |one| i64::try_from(one.as_secs()).unwrap_or(i64::MAX));
    token.expires().is_some_and(|end| end <= now)
}
