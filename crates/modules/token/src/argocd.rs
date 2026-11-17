//! The token `argocd login --sso` keeps, refreshed by `argocd account get-user-info` near its end.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use groove_exec::run::Run;
use groove_http::{Error, Result, TokenSource};
use tokio::sync::Mutex;

use crate::Cli;

/// How long before it ends a token is refreshed.
const SOON: i64 = 30;
const WAIT: Duration = Duration::from_secs(20);

/// One refresh at a time: the refresh token rotates, and two refreshes at once would race on it.
static REFRESHING: Mutex<()> = Mutex::const_new(());

/// The CLI's server and a token that lasts a while yet.
pub async fn argocd(home: &Path) -> Result<(String, groove_argocd::Token)> {
    let read = read(home).await?;
    if lasting(&read.1) {
        return Ok(read);
    }
    let _one = REFRESHING.lock().await;
    let read = self::read(home).await?;
    if lasting(&read.1) {
        return Ok(read);
    }
    let asked = Run::new(Cli::Argocd.program()).args(groove_argocd::REFRESH);
    let _ = asked.timeout(WAIT).output().await;
    let read = self::read(home).await?;
    match lasting(&read.1) {
        true => Ok(read),
        false => Err(Error::Auth(format!(
            "the argocd login has ended: {}",
            Cli::Argocd.login(&read.0)
        ))),
    }
}

/// The CLI's token as a call's source, from the config under `home`.
pub struct ArgocdToken {
    home: PathBuf,
}

impl ArgocdToken {
    pub fn new(home: impl Into<PathBuf>) -> Self {
        Self { home: home.into() }
    }
}

impl TokenSource for ArgocdToken {
    async fn token(&self) -> Result<String> {
        let (_, token) = argocd(&self.home).await?;
        Ok(token.secret().to_string())
    }

    /// Nothing is held: each call reads the config again.
    fn invalidate(&self) {}
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

/// Past `SOON` from its end, or telling no end.
fn lasting(token: &groove_argocd::Token) -> bool {
    let now = SystemTime::now().duration_since(UNIX_EPOCH);
    let now = now.map_or(0, |one| i64::try_from(one.as_secs()).unwrap_or(i64::MAX));
    token.expires().is_none_or(|end| end - now > SOON)
}
