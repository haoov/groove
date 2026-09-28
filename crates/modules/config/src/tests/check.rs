use crate::check::{logged_in, version};

#[test]
fn a_version_is_the_number_each_program_prints_first() {
    let printed = [
        ("git", "git version 2.43.0\n", "2.43.0"),
        ("gh", "gh version 2.45.0 (2024-03-04)\nhttps://…", "2.45.0"),
        ("glab", "glab 1.36.0 (a1b2c3)\n", "1.36.0"),
        (
            "curl",
            "curl 8.5.0 (x86_64-pc-linux-gnu) libcurl/8.5.0\n",
            "8.5.0",
        ),
        ("claude", "2.1.3 (Claude Code)\n", "2.1.3"),
        ("odd", "no number here\n", "no number here"),
    ];
    for (name, text, want) in printed {
        assert_eq!(version(name, text), want, "{name}");
    }
}

#[test]
fn claude_is_signed_in_only_when_its_status_says_so() {
    assert!(logged_in(
        r#"{"loggedIn": true, "authMethod": "claude.ai"}"#
    ));
    assert!(!logged_in(r#"{"loggedIn": false}"#));
    assert!(!logged_in("not json"));
}

#[tokio::test]
async fn a_program_that_does_not_run_is_not_found() {
    let tools = crate::check("/nowhere/claude").await;
    let claude = tools.iter().find(|one| one.name == "claude").unwrap();
    assert!(claude.found.is_none() && claude.required && !claude.ready());
}
