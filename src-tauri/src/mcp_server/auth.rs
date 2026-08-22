//! Per-launch bearer token for the loopback server.
//!
//! Without it, any local process could read files through `get_file_content`,
//! queue write approvals that look like agent requests, or spoof `/hook`
//! activity. Only processes we spawn receive the token — via launch FILES, never
//! argv, because `/proc/<pid>/cmdline` is world-readable.

use std::sync::OnceLock;

use axum::response::IntoResponse;

/// The token, generated once per app launch. Never persisted.
pub(crate) fn token() -> &'static str {
    static TOKEN: OnceLock<String> = OnceLock::new();
    TOKEN.get_or_init(|| {
        format!("{}{}", uuid::Uuid::new_v4().simple(), uuid::Uuid::new_v4().simple())
    })
}

/// `Authorization: Bearer <token>` or `?token=<token>` — the query form exists
/// for clients that cannot set headers on an SSE connection.
fn authorized(auth_header: Option<&str>, query: Option<&str>) -> bool {
    let expected = token();
    if let Some(bearer) = auth_header.and_then(|a| {
        a.strip_prefix("Bearer ").or_else(|| a.strip_prefix("bearer "))
    }) {
        if bearer.trim() == expected {
            return true;
        }
    }
    query.is_some_and(|q| {
        q.split('&')
            .any(|pair| pair.strip_prefix("token=").is_some_and(|v| v == expected))
    })
}

pub(super) async fn require_auth(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let ok = authorized(
        req.headers()
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok()),
        req.uri().query(),
    );
    if ok {
        next.run(req).await
    } else {
        // A failed check is either a stale agent from a previous launch or a
        // foreign local process probing the port — both worth a trace.
        tracing::warn!("[mcp] unauthorized request to {}", req.uri().path());
        axum::http::StatusCode::UNAUTHORIZED.into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::{authorized, token};

    #[test]
    fn only_the_launch_token_passes() {
        let t = token();
        assert!(authorized(Some(&format!("Bearer {t}")), None));
        assert!(authorized(None, Some(&format!("task=X&token={t}"))));

        assert!(!authorized(None, None), "no credential");
        assert!(!authorized(Some("Bearer nope"), None), "wrong bearer");
        assert!(!authorized(None, Some("token=nope")), "wrong query token");
        assert!(!authorized(Some(t), None), "raw token without the Bearer scheme");
    }
}
