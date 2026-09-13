use crate::redact::redact_url;

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
