//! Where a token comes from, and the header it becomes.

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
