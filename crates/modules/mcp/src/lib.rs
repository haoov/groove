//! The agent's tool server, MCP over HTTP and SSE: `GET /sse?task=` streams, `POST /message?sessionId=` calls.

mod call;
mod rpc;
mod stream;

#[cfg(test)]
mod tests;

use std::sync::Arc;

use groove_loopback::{body, reply};
use groove_types::{Error, ErrorKind, Result};
use hyper::body::Incoming;
use hyper::{Method, Request, Response, StatusCode};
use tokio::runtime::Handle;

use stream::{Body, Connections, param};

pub use call::{Answer, Call, Reply};
#[cfg(test)]
use groove_loopback::BODY_MAX;

/// Where an agent reaches its tools, and the token it must carry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Server {
    pub port: u16,
    pub token: String,
}

impl Server {
    /// The url an agent for this session connects to.
    pub fn sse_url(&self, session: &str) -> String {
        format!("http://127.0.0.1:{}/sse?task={session}", self.port)
    }
}

type Sink = Arc<dyn Fn(Call) + Send + Sync>;

/// Takes a free loopback port and serves it until the process ends.
pub fn serve(handle: &Handle, on_call: impl Fn(Call) + Send + Sync + 'static) -> Result<Server> {
    let (live, sink): (Connections, Sink) = (Connections::default(), Arc::new(on_call));
    let bound = groove_loopback::serve(handle, move |request| {
        let (live, sink) = (live.clone(), sink.clone());
        async move { route(request, &live, &sink).await }
    })
    .map_err(|e| Error::new(ErrorKind::Agent, format!("no loopback port: {e}")))?;
    Ok(Server {
        port: bound.port,
        token: bound.token,
    })
}

/// The two routes: the stream that answers, and the calls that ask.
async fn route(request: Request<Incoming>, live: &Connections, sink: &Sink) -> Response<Body> {
    match (request.method(), request.uri().path()) {
        (&Method::GET, "/sse") => stream::open(live, request.uri().query()),
        (&Method::POST, "/message") => message(request, live, sink).await,
        _ => reply(StatusCode::NOT_FOUND),
    }
}

/// One JSON-RPC message. The answer goes back on the stream, never on this response.
async fn message(request: Request<Incoming>, live: &Connections, sink: &Sink) -> Response<Body> {
    let Some(id) = param(request.uri().query(), "sessionId") else {
        return reply(StatusCode::BAD_REQUEST);
    };
    let Some(open) = live.open(&id) else {
        return reply(StatusCode::NOT_FOUND);
    };
    let Some(body) = body(request).await else {
        return reply(StatusCode::PAYLOAD_TOO_LARGE);
    };
    let Ok(call) = serde_json::from_slice::<serde_json::Value>(&body) else {
        return reply(StatusCode::BAD_REQUEST);
    };
    if call.get("id").is_none_or(serde_json::Value::is_null) {
        return reply(StatusCode::ACCEPTED);
    }
    let sink = sink.clone();
    tokio::spawn(async move {
        open.send(&rpc::answer(&call, &open.session, &sink).await)
            .await
    });
    reply(StatusCode::ACCEPTED)
}
