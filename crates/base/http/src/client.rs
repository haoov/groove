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
