use groove_http::TokenSource;

use crate::{Cli, Token};

/// What `glab auth status --show-token` prints for one host.
const REPORT: &str = "\
gitlab.example.com
  ✓ Logged in to gitlab.example.com as someone (/home/someone/.config/glab-cli/config.yml)
  ✓ REST API Endpoint: https://gitlab.example.com/api/v4/
  ✓ Token found in configuration file (plaintext): glpat-a-token
  ! To store this token more securely, run glab auth login --hostname gitlab.example.com";

#[test]
fn gh_prints_the_token_and_nothing_else() {
    assert_eq!(Cli::Gh.read("gho_a-token\n"), Some("gho_a-token".into()));
}

#[test]
fn glab_prints_a_report_the_token_stands_in() {
    assert_eq!(Cli::Glab.read(REPORT), Some("glpat-a-token".into()));
}

#[test]
fn the_line_advising_a_login_is_not_a_token() {
    let logged_out = "gitlab.example.com\n  x Not logged in. Run glab auth login for a token";
    assert_eq!(Cli::Glab.read(logged_out), None);
}

#[test]
fn a_tool_that_printed_nothing_holds_no_token() {
    assert_eq!(Cli::Gh.read("  \n"), None);
    assert_eq!(Cli::Glab.read(""), None);
}

#[tokio::test]
async fn a_token_handed_over_is_the_one_used() {
    let token = Token::Fixed("t".into());
    assert_eq!(token.token().await.ok(), Some("t".into()));
    token.invalidate();
    assert_eq!(
        token.token().await.ok(),
        Some("t".into()),
        "nothing to drop"
    );
}

#[test]
fn a_configured_token_wins_over_the_cli() {
    let token = Token::configured(Some("t".into()), Cli::Gh, "github.com");
    assert!(matches!(token, Token::Fixed(_)));
    let asked = Token::configured(None, Cli::Gh, "github.com");
    assert!(matches!(asked, Token::Cli(_)));
}

/// A home whose argocd config holds `token` for the CLI's current context.
fn argocd_home(token: &str) -> tempfile::TempDir {
    let home = tempfile::tempdir().unwrap();
    let dir = home.path().join(".config/argocd");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("config"), config_with(token)).unwrap();
    home
}

/// The CLI's config holding `token` for its current context.
fn config_with(token: &str) -> String {
    format!(
        "contexts:\n- name: argo.example.com\n  server: argo.example.com\n  user: argo.example.com\n\
         current-context: argo.example.com\nusers:\n- auth-token: {token}\n  name: argo.example.com\n"
    )
}

#[tokio::test]
async fn a_lasting_argocd_token_is_read_from_its_config_without_the_cli() {
    let lasting = "e30.eyJleHAiOjk5OTk5OTk5OTl9.c2ln";
    let home = argocd_home(lasting);
    let (server, token) = crate::argocd(home.path()).await.unwrap();
    assert_eq!(server, "argo.example.com");
    assert_eq!(token.secret(), lasting);
}

#[tokio::test]
async fn no_argocd_login_says_how_to_log_in() {
    let home = tempfile::tempdir().unwrap();
    let refused = crate::argocd(home.path()).await.unwrap_err().to_string();
    assert!(refused.contains("argocd login"), "{refused}");
}

/// A stand-in for the CLI that writes `token` into the config under `home`.
fn refreshing(home: &std::path::Path, token: &str) -> String {
    let fresh = home.join("fresh");
    std::fs::write(&fresh, config_with(token)).unwrap();
    let config = home.join(".config/argocd/config");
    let script = home.join("argocd");
    let body = format!(
        "#!/bin/sh\ncp '{}' '{}'\n",
        fresh.display(),
        config.display()
    );
    std::fs::write(&script, body).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    script.display().to_string()
}

async fn used(home: &std::path::Path) -> groove_argocd::Token {
    crate::argocd(home).await.unwrap().1
}

#[tokio::test]
async fn a_refused_token_is_refreshed_by_the_cli_and_read_again() {
    let home = argocd_home("old");
    let cli = refreshing(home.path(), "new");
    let refused = used(home.path()).await;
    let (_, token) = crate::argocd::refreshed(home.path(), &refused, &cli)
        .await
        .unwrap();
    assert_eq!(token.secret(), "new");
}

#[tokio::test]
async fn a_token_another_call_refreshed_is_taken_without_the_cli() {
    let home = argocd_home("old");
    let refused = used(home.path()).await;
    std::fs::write(
        home.path().join(".config/argocd/config"),
        config_with("new"),
    )
    .unwrap();
    let (_, token) = crate::argocd::refreshed(home.path(), &refused, "/nonexistent/argocd")
        .await
        .unwrap();
    assert_eq!(token.secret(), "new");
}

#[tokio::test]
async fn a_refresh_that_changes_nothing_says_what_the_cli_said_and_how_to_log_in() {
    let home = argocd_home("old");
    let refused = used(home.path()).await;
    let failed = crate::argocd::refreshed(home.path(), &refused, "false")
        .await
        .unwrap_err()
        .to_string();
    assert!(failed.contains("the argocd login has ended"), "{failed}");
    assert!(
        failed.contains("argocd login argo.example.com --sso"),
        "{failed}"
    );
}
