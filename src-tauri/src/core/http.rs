//! The process-wide HTTP client. One client means one connection pool: every
//! request to the same host reuses a live TLS connection instead of paying a
//! fresh handshake.

use std::sync::OnceLock;
use std::time::Duration;

pub fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .build()
            .expect("HTTP client build cannot fail with a static config")
    })
}

#[cfg(test)]
mod tests {
    /// The client above is the only one — a second pool is a second set of
    /// handshakes. Same style as the git-spawn and raw-SQL guards.
    #[test]
    fn no_http_client_outside_core_http() {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut offenders = vec![];
        visit(&src, &mut offenders);
        assert!(offenders.is_empty(), "reqwest::Client built outside core/http: {offenders:?}");

        fn visit(dir: &std::path::Path, offenders: &mut Vec<String>) {
            for entry in std::fs::read_dir(dir).unwrap().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    visit(&path, offenders);
                } else if path.extension().is_some_and(|e| e == "rs")
                    && !path.to_string_lossy().contains("/core/http.rs")
                    && std::fs::read_to_string(&path)
                        .unwrap()
                        .contains("reqwest::Client::")
                {
                    offenders.push(path.display().to_string());
                }
            }
        }
    }
}
