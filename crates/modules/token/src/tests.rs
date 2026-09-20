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
