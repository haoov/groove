use crate::token_of;

const CONFIG: &str = "\
contexts:
- name: argo.example.com
  server: argo.example.com
  user: argo.example.com
- name: other
  server: other.example.com
  user: other
current-context: argo.example.com
prompts-enabled: false
servers:
- grpc-web-root-path: \"\"
  server: argo.example.com
users:
- auth-token: secret-token
  name: argo.example.com
  refresh-token: refresh
- auth-token: wrong
  name: other
";

#[test]
fn the_current_context_gives_its_server_and_its_user_s_token() {
    let (server, token) = token_of(CONFIG).expect("a token");
    assert_eq!(server, "argo.example.com");
    assert_eq!(token.cookie(), "argocd.token=secret-token");
}

#[test]
fn a_token_never_prints() {
    let (_, token) = token_of(CONFIG).expect("a token");
    assert!(!format!("{token:?}").contains("secret"));
}

#[test]
fn no_login_gives_no_token() {
    let signed_out = CONFIG
        .replace("- auth-token: secret-token\n", "- name: argo.example.com\n")
        .replacen(
            "  name: argo.example.com\n  refresh-token",
            "  refresh-token",
            1,
        );
    assert!(token_of(&signed_out).is_none());
    assert!(token_of("").is_none());
}
