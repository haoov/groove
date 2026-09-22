use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::{Graphql, Method, Result, TokenSource};

/// One token, handed over as often as it is asked for.
struct Held;

impl TokenSource for Held {
    async fn token(&self) -> Result<String> {
        Ok("t".to_string())
    }
    fn invalidate(&self) {}
}

/// An endpoint whose host also answers a plain call.
fn endpoint(server: &MockServer) -> Graphql<Held> {
    let url = format!("http://{}/api/graphql", server.address());
    Graphql::new(url, Held).expect("a client")
}

#[tokio::test]
async fn a_plain_call_carries_the_endpoints_own_token() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v4/approve"))
        .respond_with(ResponseTemplate::new(201))
        .mount(&server)
        .await;
    let api = endpoint(&server);
    let url = format!("http://{}/api/v4/approve", server.address());

    api.beside().post(&url).await.expect("the call is made");
    let sent = server.received_requests().await.unwrap_or_default();
    let one = sent.first().expect("one call");
    assert_eq!(one.method, Method::POST);
    assert_eq!(
        one.headers
            .get("authorization")
            .map(|v| v.to_str().unwrap()),
        Some("Bearer t")
    );
}

#[tokio::test]
async fn a_refused_plain_call_is_an_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(403).set_body_string("not allowed"))
        .mount(&server)
        .await;
    let api = endpoint(&server);
    let url = format!("http://{}/api/v4/approve", server.address());
    let refused = api.beside().post(&url).await.expect_err("403 is an error");
    assert!(format!("{refused}").contains("403"), "{refused}");
}
