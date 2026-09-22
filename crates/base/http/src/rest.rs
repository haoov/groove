//! The plain calls beside a GraphQL endpoint: what a host keeps out of its schema.

use crate::{Client, Method, Result, TokenSource};

/// One host's REST calls, on the client and the token its endpoint already holds.
pub struct Rest<'a, T: TokenSource> {
    client: &'a Client,
    token: &'a T,
}

impl<'a, T: TokenSource> Rest<'a, T> {
    pub(crate) fn new(client: &'a Client, token: &'a T) -> Self {
        Self { client, token }
    }

    /// A call whose answer nothing reads: it either happened or it is an error.
    pub async fn post(&self, url: &str) -> Result<()> {
        self.client
            .request(Method::POST, url)
            .send_authed(self.token)
            .await?
            .ok()?;
        Ok(())
    }
}
