//! The loopback the agent's hooks post to: `POST /hook/<session>` on 127.0.0.1, behind a bearer token.

mod parse;

#[cfg(test)]
mod tests;

use std::sync::Arc;

use groove_loopback::{body, reply};
use groove_types::{Error, ErrorKind, Result};
use http_body_util::Full;
use hyper::body::{Bytes, Incoming};
use hyper::{Method, Request, Response, StatusCode};
use tokio::runtime::Handle;

pub use parse::Post;

/// Where the agent posts its hooks, and the token it must carry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Receiver {
    pub port: u16,
    pub token: String,
}

impl Receiver {
    /// The url an agent for this session posts to.
    pub fn hook_url(&self, session: &str) -> String {
        format!("http://127.0.0.1:{}/hook/{session}", self.port)
    }
}

type Sink = Arc<dyn Fn(Post) + Send + Sync>;

/// Takes a free loopback port and serves it until the process ends.
pub fn serve(handle: &Handle, on_post: impl Fn(Post) + Send + Sync + 'static) -> Result<Receiver> {
    let sink: Sink = Arc::new(on_post);
    let bound = groove_loopback::serve(handle, move |request| {
        let sink = sink.clone();
        async move { route(request, &sink).await }
    })
    .map_err(|e| Error::new(ErrorKind::Agent, format!("no loopback port: {e}")))?;
    Ok(Receiver {
        port: bound.port,
        token: bound.token,
    })
}

/// The only route there is.
async fn route(request: Request<Incoming>, sink: &Sink) -> Response<Full<Bytes>> {
    if request.method() != Method::POST {
        return reply(StatusCode::METHOD_NOT_ALLOWED);
    }
    let Some(session) = session_of(request.uri().path()) else {
        return reply(StatusCode::NOT_FOUND);
    };
    let Some(body) = body(request).await else {
        return reply(StatusCode::PAYLOAD_TOO_LARGE);
    };
    match parse::post(&session, &body) {
        Some(post) => {
            sink(post);
            reply(StatusCode::NO_CONTENT)
        }
        None => reply(StatusCode::BAD_REQUEST),
    }
}

/// `/hook/<session>`, and nothing shorter or longer.
fn session_of(path: &str) -> Option<String> {
    let rest = path.strip_prefix("/hook/")?;
    let session = rest.trim_end_matches('/');
    (!session.is_empty() && !session.contains('/')).then(|| session.to_string())
}
