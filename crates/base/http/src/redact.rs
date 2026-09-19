//! What never reaches a log: tokens, and the URLs that carry them.

/// The URL without its user and password, for messages and logs.
pub fn redact_url(url: &str) -> String {
    let Ok(mut parsed) = reqwest::Url::parse(url) else {
        return url.to_string();
    };
    if parsed.username().is_empty() && parsed.password().is_none() {
        return url.to_string();
    }
    let _ = parsed.set_username("");
    let _ = parsed.set_password(None);
    parsed.to_string().replacen("://", "://<redacted>@", 1)
}
