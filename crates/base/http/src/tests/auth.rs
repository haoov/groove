use std::sync::Mutex;

use crate::Result;
use crate::auth::*;
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
