use crate::redact::redact;

#[test]
fn credentials_never_survive_redaction() {
    assert_eq!(
        redact("fatal: unable to access 'https://oauth2:glpat-xyz@gitlab.example.com/g/p.git/'"),
        "fatal: unable to access 'https://<redacted>@gitlab.example.com/g/p.git/'"
    );
    assert_eq!(
        redact("remote: https://token@github.com/o/r"),
        "remote: https://<redacted>@github.com/o/r"
    );
    assert_eq!(
        redact("ssh://git@host:2222/g/p"),
        "ssh://<redacted>@host:2222/g/p"
    );
}

#[test]
fn text_without_credentials_is_untouched() {
    assert_eq!(
        redact("cloned https://github.com/o/r"),
        "cloned https://github.com/o/r"
    );
    assert_eq!(redact("nothing to commit"), "nothing to commit");
}
