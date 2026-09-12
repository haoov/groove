use std::time::Duration;

use crate::{Method, Response, Result, redact_url};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const TIMEOUT: Duration = Duration::from_secs(30);

/// One client, one connection pool. Cheap to clone; every clone shares the pool.
#[derive(Clone)]
pub struct Client {
    inner: reqwest::Client,
}

impl Client {
    pub fn new() -> Result<Self> {
        let inner = reqwest::Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(TIMEOUT)
            .user_agent("groove")
            .build()?;
        Ok(Self { inner })
    }

    pub fn request(&self, method: Method, url: impl Into<String>) -> Request {
        Request {
            client: self.clone(),
            method,
            url: url.into(),
            headers: Vec::new(),
            bearer: None,
            body: None,
        }
    }
}

#[derive(Clone)]
pub struct Request {
    client: Client,
    method: Method,
    url: String,
    headers: Vec<(String, String)>,
    bearer: Option<String>,
    body: Option<serde_json::Value>,
}

impl Request {
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((key.into(), value.into()));
        self
    }

    pub fn bearer(mut self, token: impl Into<String>) -> Self {
        self.bearer = Some(token.into());
        self
    }

    pub fn json(mut self, body: &serde_json::Value) -> Self {
        self.body = Some(body.clone());
        self
    }

    pub fn method(&self) -> &Method {
        &self.method
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    /// Sends and returns the response whatever its status.
    pub async fn send(self) -> Result<Response> {
        let mut builder = self.client.inner.request(self.method.clone(), &self.url);
        for (key, value) in &self.headers {
            builder = builder.header(key, value);
        }
        if let Some(token) = &self.bearer {
            builder = builder.bearer_auth(token);
        }
        if let Some(body) = &self.body {
            builder = builder.json(body);
        }
        let reply = builder.send().await?;
        Ok(Response {
            method: self.method.to_string(),
            url: redact_url(&self.url),
            status: reply.status(),
            body: reply.text().await.unwrap_or_default(),
        })
    }

    /// Sends and decodes a successful JSON reply; any failure status is an error.
    pub async fn send_json<T: serde::de::DeserializeOwned>(self) -> Result<T> {
        self.send().await?.ok()?.json()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn headers_bearer_and_body_reach_the_server() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/pages"))
            .and(header("authorization", "Bearer t0k"))
            .and(header("notion-version", "2022-06-28"))
            .and(body_json(serde_json::json!({"a": 1})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "p1"})))
            .mount(&server)
            .await;

        let client = Client::new().unwrap();
        let reply: serde_json::Value = client
            .request(Method::POST, format!("{}/v1/pages", server.uri()))
            .bearer("t0k")
            .header("Notion-Version", "2022-06-28")
            .json(&serde_json::json!({"a": 1}))
            .send_json()
            .await
            .unwrap();
        assert_eq!(reply["id"], "p1");
    }

    #[tokio::test]
    async fn a_failure_status_is_an_error_with_the_message() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(
                ResponseTemplate::new(422).set_body_json(serde_json::json!({"message": "nope"})),
            )
            .mount(&server)
            .await;

        let err = Client::new()
            .unwrap()
            .request(Method::GET, format!("{}/x", server.uri()))
            .send_json::<serde_json::Value>()
            .await
            .unwrap_err();
        assert!(
            matches!(err, crate::Error::Status { status: 422, .. }),
            "{err}"
        );
        assert!(err.to_string().ends_with("422: nope"), "{err}");
    }
}
