//! A GraphQL endpoint: one call, and the errors a 200 can carry.

use crate::{Client, Error, Method, Result, TokenSource};

/// One endpoint, called with one token.
pub struct Graphql<T: TokenSource> {
    client: Client,
    url: String,
    token: T,
}

impl<T: TokenSource> Graphql<T> {
    pub fn new(url: impl Into<String>, token: T) -> Result<Self> {
        Ok(Self {
            client: Client::new()?,
            url: url.into(),
            token,
        })
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    /// One query with its variables; the errors a 200 carries become an error.
    pub async fn ask(
        &self,
        query: &str,
        variables: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let body = serde_json::json!({ "query": query, "variables": variables });
        let reply: serde_json::Value = self
            .client
            .request(Method::POST, &self.url)
            .json(&body)
            .send_authed_json(&self.token)
            .await?;
        match refusal(&reply) {
            Some(message) => Err(Error::Refused {
                url: self.url.clone(),
                message,
            }),
            None => Ok(reply),
        }
    }
}

/// Every message in the reply's `errors`, when it carries any.
fn refusal(reply: &serde_json::Value) -> Option<String> {
    let errors = reply["errors"].as_array()?;
    let said: Vec<&str> = errors
        .iter()
        .filter_map(|one| one["message"].as_str())
        .collect();
    (!said.is_empty()).then(|| said.join("; "))
}
