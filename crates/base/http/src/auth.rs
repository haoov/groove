use crate::{Request, Response, Result, StatusCode};

/// Where a bearer token comes from, and how to drop one the server refused.
pub trait TokenSource {
    fn token(&self) -> impl std::future::Future<Output = Result<String>> + Send;
    fn invalidate(&self);
}

impl Request {
    /// Sends with a token from `source`; on 401, drops that token and tries once more.
    pub async fn send_authed(self, source: &impl TokenSource) -> Result<Response> {
        let first = self.clone().bearer(source.token().await?).send().await?;
        if first.status != StatusCode::UNAUTHORIZED {
            return Ok(first);
        }
        source.invalidate();
        self.bearer(source.token().await?).send().await
    }

    /// `send_authed`, then a successful JSON reply or an error.
    pub async fn send_authed_json<T: serde::de::DeserializeOwned>(
        self,
        source: &impl TokenSource,
    ) -> Result<T> {
        self.send_authed(source).await?.ok()?.json()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::{Client, Error, Method};
    use wiremock::matchers::{header, method};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    struct Tokens {
        issued: Mutex<Vec<&'static str>>,
        invalidated: Mutex<usize>,
    }

    impl TokenSource for Tokens {
        async fn token(&self) -> Result<String> {
            Ok(self.issued.lock().unwrap().remove(0).to_string())
        }
        fn invalidate(&self) {
            *self.invalidated.lock().unwrap() += 1;
        }
    }

    #[tokio::test]
    async fn a_refused_token_is_dropped_and_the_call_retried_once() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(header("authorization", "Bearer stale"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(header("authorization", "Bearer fresh"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))
            .mount(&server)
            .await;
        let tokens = Tokens {
            issued: Mutex::new(vec!["stale", "fresh"]),
            invalidated: Mutex::new(0),
        };

        let reply: serde_json::Value = Client::new()
            .unwrap()
            .request(Method::GET, format!("{}/me", server.uri()))
            .send_authed_json(&tokens)
            .await
            .unwrap();
        assert_eq!(reply["ok"], true);
        assert_eq!(*tokens.invalidated.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn a_second_refusal_is_reported_as_unauthorized() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;
        let tokens = Tokens {
            issued: Mutex::new(vec!["a", "b"]),
            invalidated: Mutex::new(0),
        };

        let err = Client::new()
            .unwrap()
            .request(Method::GET, format!("{}/me", server.uri()))
            .send_authed_json::<serde_json::Value>(&tokens)
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Unauthorized { .. }), "{err}");
    }
}
