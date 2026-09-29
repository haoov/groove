//! A web page opened in the user's browser.

use groove_exec::run::Run;
use groove_types::{Error, ErrorKind, Result};

#[cfg(target_os = "macos")]
const OPENER: &str = "open";
#[cfg(not(target_os = "macos"))]
const OPENER: &str = "xdg-open";

/// Hands `url` to the desktop's opener; only a web address is handed over.
pub async fn browse(url: &str) -> Result<()> {
    if !url.starts_with("https://") && !url.starts_with("http://") {
        return Err(Error::invalid(format!("not a web address: {url}")));
    }
    let opened = Run::new(OPENER).args([url]).text().await;
    opened
        .map(|_| ())
        .map_err(|e| Error::new(ErrorKind::Io, e.to_string()))
}
