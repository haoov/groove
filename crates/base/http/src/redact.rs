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

#[cfg(test)]
mod tests {
    use super::redact_url;

    #[test]
    fn credentials_leave_the_url() {
        assert_eq!(
            redact_url("https://oauth2:glpat-xyz@gitlab.example.com/api/v4/p"),
            "https://<redacted>@gitlab.example.com/api/v4/p"
        );
    }

    #[test]
    fn a_plain_url_is_untouched() {
        assert_eq!(
            redact_url("https://api.notion.com/v1/pages"),
            "https://api.notion.com/v1/pages"
        );
        assert_eq!(redact_url("not a url"), "not a url");
    }
}
