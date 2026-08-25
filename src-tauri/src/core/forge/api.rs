//! Direct HTTP to the forges, on the shared `core::http` client.
//!
//! Replaces shelling out to `glab api` / `gh api`: JSON bodies cannot
//! `@`-expand into local file reads the way CLI form fields do, a kept-alive
//! connection beats a process spawn + TLS handshake per call, and errors are
//! status codes instead of stderr archaeology. The CLIs remain the token source
//! (see `auth`).

use crate::core::timing::timed;

use super::Platform;

/// One request, with a single retry after a 401 (token rotated or re-issued —
/// forget the cached one and ask the CLI again).
async fn call(
    platform: Platform,
    host: &str,
    method: reqwest::Method,
    url: &str,
    body: Option<&serde_json::Value>,
) -> anyhow::Result<serde_json::Value> {
    let first = send(platform, host, method.clone(), url, body).await?;
    if first.status != reqwest::StatusCode::UNAUTHORIZED {
        return finish(first, method.as_str(), url);
    }
    super::auth::forget(host);
    let second = send(platform, host, method.clone(), url, body).await?;
    finish(second, method.as_str(), url)
}

struct Reply {
    status: reqwest::StatusCode,
    body: String,
}

async fn send(
    platform: Platform,
    host: &str,
    method: reqwest::Method,
    url: &str,
    body: Option<&serde_json::Value>,
) -> anyhow::Result<Reply> {
    let token = super::auth::token(platform, host).await?;
    let verb = method.as_str().to_string();
    timed("http", format!("forge {verb} {url}"), async {
        let mut req = crate::core::http::client()
            .request(method, url)
            .bearer_auth(token)
            .header("User-Agent", "groove");
        if let Some(body) = body {
            req = req.json(body);
        }
        let resp = req.send().await?;
        Ok(Reply {
            status: resp.status(),
            body: resp.text().await.unwrap_or_default(),
        })
    })
    .await
}

fn finish(reply: Reply, verb: &str, url: &str) -> anyhow::Result<serde_json::Value> {
    if !reply.status.is_success() {
        let detail = api_error(&reply.body);
        if reply.status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(anyhow::anyhow!(
                "the forge rejected the token ({verb} {url}) — re-run the CLI login"
            ));
        }
        return Err(anyhow::anyhow!("{verb} {url} failed {}: {detail}", reply.status));
    }
    if reply.body.trim().is_empty() {
        return Ok(serde_json::Value::Null);
    }
    Ok(serde_json::from_str(&reply.body)?)
}

/// The human part of an API error body, whatever key this forge put it under.
fn api_error(body: &str) -> String {
    let v: serde_json::Value = serde_json::from_str(body).unwrap_or_default();
    v["message"]
        .as_str()
        .map(str::to_string)
        .or_else(|| v["message"].as_array().map(|a| {
            a.iter().filter_map(|m| m.as_str()).collect::<Vec<_>>().join("; ")
        }))
        .or_else(|| v["error"].as_str().map(str::to_string))
        .unwrap_or_else(|| {
            let flat = body.trim();
            let mut short: String = flat.chars().take(300).collect();
            if flat.chars().count() > 300 {
                short.push('…');
            }
            short
        })
}

// ─── GitLab (REST v4) ─────────────────────────────────────────────────────────

/// `path` is relative to `/api/v4/`, already query-encoded by the caller.
pub(crate) async fn gitlab(
    host: &str,
    method: reqwest::Method,
    path: &str,
    body: Option<&serde_json::Value>,
) -> anyhow::Result<serde_json::Value> {
    let url = format!("https://{host}/api/v4/{path}");
    call(Platform::Gitlab, host, method, &url, body).await
}

/// A project's URL-encoded full path — the `{id}` of every project endpoint.
pub(crate) fn gitlab_project_ref(group_path: &str, project: &str) -> String {
    format!("{group_path}/{project}").replace('/', "%2F")
}

/// Percent-encode one query-string VALUE (branch names, usernames). Everything
/// outside the RFC 3986 unreserved set is encoded, so `&`, `=`, `#`, `+` and
/// spaces cannot restructure the query.
pub(crate) fn pct(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for b in value.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

// ─── GitHub (REST + GraphQL) ──────────────────────────────────────────────────

fn github_rest_base(host: &str) -> String {
    if host == "github.com" {
        "https://api.github.com".to_string()
    } else {
        format!("https://{host}/api/v3")
    }
}

fn github_graphql_url(host: &str) -> String {
    if host == "github.com" {
        "https://api.github.com/graphql".to_string()
    } else {
        format!("https://{host}/api/graphql")
    }
}

/// `path` is relative to the REST root, e.g. `repos/{owner}/{repo}/pulls/1`.
pub(crate) async fn github(
    host: &str,
    method: reqwest::Method,
    path: &str,
    body: Option<&serde_json::Value>,
) -> anyhow::Result<serde_json::Value> {
    let url = format!("{}/{path}", github_rest_base(host));
    call(Platform::Github, host, method, &url, body).await
}

/// One GraphQL call. Variables are real JSON, so types survive without the
/// CLI's `-f`/`-F` string-vs-typed dance.
pub(crate) async fn github_graphql(
    host: &str,
    query: &str,
    variables: serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    let url = github_graphql_url(host);
    let body = serde_json::json!({ "query": query, "variables": variables });
    let v = call(Platform::Github, host, reqwest::Method::POST, &url, Some(&body)).await?;
    if let Some(errors) = v["errors"].as_array() {
        if !errors.is_empty() {
            let msg = errors
                .iter()
                .filter_map(|e| e["message"].as_str())
                .collect::<Vec<_>>()
                .join("; ");
            return Err(anyhow::anyhow!("GitHub GraphQL: {msg}"));
        }
    }
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_ref_is_url_encoded() {
        assert_eq!(gitlab_project_ref("wiremind/devops", "cm"), "wiremind%2Fdevops%2Fcm");
    }

    #[test]
    fn query_values_cannot_restructure_the_query() {
        assert_eq!(pct("feat/x-1"), "feat%2Fx-1");
        assert_eq!(pct("a&b=c #+"), "a%26b%3Dc%20%23%2B");
        assert_eq!(pct("plain-1.2_x~"), "plain-1.2_x~");
    }

    #[test]
    fn github_hosts_map_to_their_api_roots() {
        assert_eq!(github_rest_base("github.com"), "https://api.github.com");
        assert_eq!(github_rest_base("ghe.corp.io"), "https://ghe.corp.io/api/v3");
        assert_eq!(github_graphql_url("github.com"), "https://api.github.com/graphql");
        assert_eq!(github_graphql_url("ghe.corp.io"), "https://ghe.corp.io/api/graphql");
    }

    /// GitLab reports `message` as a string OR an array; GitHub as a string.
    #[test]
    fn error_bodies_reduce_to_their_message() {
        assert_eq!(api_error(r#"{"message":"404 Not Found"}"#), "404 Not Found");
        assert_eq!(api_error(r#"{"message":["a is bad","b too"]}"#), "a is bad; b too");
        assert_eq!(api_error(r#"{"error":"invalid_token"}"#), "invalid_token");
        assert_eq!(api_error("plain text"), "plain text");
    }

    /// The API host may be named nowhere else: outside this file, GitHub is
    /// reached through named functions only. Same style as the other guards.
    #[test]
    fn no_github_api_outside_core_forge_api() {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut offenders = vec![];
        visit(&src, &mut offenders);
        assert!(offenders.is_empty(), "api.github.com outside core/forge/api.rs: {offenders:?}");

        fn visit(dir: &std::path::Path, offenders: &mut Vec<String>) {
            for entry in std::fs::read_dir(dir).unwrap().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    visit(&path, offenders);
                } else if path.extension().is_some_and(|e| e == "rs")
                    && !path.to_string_lossy().ends_with("/core/forge/api.rs")
                    && std::fs::read_to_string(&path).unwrap().contains("api.github.com")
                {
                    offenders.push(path.display().to_string());
                }
            }
        }
    }
}
