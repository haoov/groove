//! The agent's tool server, MCP over HTTP and SSE: `GET /sse?task=` streams, `POST /message?sessionId=` calls.

mod call;
mod rpc;
mod stream;

#[cfg(test)]
mod tests;

use std::convert::Infallible;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;

use groove_types::{Error, ErrorKind, Result};
use http_body_util::{BodyExt, Limited};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use tokio::runtime::Handle;

use stream::{Body, Connections, param, reply};

pub use call::{Answer, Call, Reply};

const BODY_MAX: usize = 1 << 20;

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
    let listener = handle.block_on(bind())?;
    let port = listener.local_addr().map_err(failed)?.port();
    let token = uuid::Uuid::new_v4().simple().to_string();
    handle.spawn(accept(listener, token.clone(), Arc::new(on_call)));
    Ok(Server { port, token })
}

async fn bind() -> Result<TcpListener> {
    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
    TcpListener::bind(address).await.map_err(failed)
}

fn failed(e: std::io::Error) -> Error {
    Error::new(ErrorKind::Agent, format!("no loopback port: {e}"))
}

/// One connection at a time off the socket, each served on its own task.
async fn accept(listener: TcpListener, token: String, sink: Sink) {
    let live = Connections::default();
    loop {
        let Ok((stream, _)) = listener.accept().await else {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            continue;
        };
        let (token, live, sink) = (token.clone(), live.clone(), sink.clone());
        tokio::spawn(async move {
            let service = service_fn(move |request| {
                let (token, live, sink) = (token.clone(), live.clone(), sink.clone());
                async move { Ok::<_, Infallible>(route(request, &token, &live, &sink).await) }
            });
            let _ = http1::Builder::new()
                .serve_connection(TokioIo::new(stream), service)
                .await;
        });
    }
}

/// The two routes: the stream that answers, and the calls that ask.
async fn route(
    request: Request<Incoming>,
    token: &str,
    live: &Connections,
    sink: &Sink,
) -> Response<Body> {
    if !bearer(&request, token) {
        return reply(StatusCode::UNAUTHORIZED);
    }
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
    let Ok(body) = Limited::new(request.into_body(), BODY_MAX).collect().await else {
        return reply(StatusCode::PAYLOAD_TOO_LARGE);
    };
    let Ok(call) = serde_json::from_slice::<serde_json::Value>(&body.to_bytes()) else {
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

fn bearer(request: &Request<Incoming>, token: &str) -> bool {
    request
        .headers()
        .get(hyper::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .is_some_and(|value| value == token)
}
