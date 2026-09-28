//! A forge page opened in the user's browser.

use groove_exec::run::Run;

use crate::Result;

#[cfg(target_os = "macos")]
const OPENER: &str = "open";
#[cfg(not(target_os = "macos"))]
const OPENER: &str = "xdg-open";

/// Hands `url` to the desktop's opener; only a web address is handed over.
pub async fn browse(url: &str) -> Result<()> {
    if !url.starts_with("https://") && !url.starts_with("http://") {
        return Err(crate::Error::Invalid(format!("not a web address: {url}")));
    }
    Run::new(OPENER).args([url]).text().await?;
    Ok(())
}
