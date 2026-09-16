//! The loopback receiver: the agent's hooks come back here and nowhere else.
//! One route, `POST /hook/<session>`, on 127.0.0.1, behind a bearer token.

mod parse;

#[cfg(test)]
mod tests;

use std::convert::Infallible;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;

use groove_types::{Error, ErrorKind, Result};
use http_body_util::{BodyExt, Full, Limited};
use hyper::body::{Bytes, Incoming};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use tokio::runtime::Handle;

pub use parse::Post;

const BODY_MAX: usize = 1 << 20;

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
    let listener = handle.block_on(bind())?;
    let port = listener.local_addr().map_err(failed)?.port();
    let token = uuid::Uuid::new_v4().simple().to_string();
    let sink: Sink = Arc::new(on_post);
    handle.spawn(accept(listener, token.clone(), sink));
    Ok(Receiver { port, token })
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
    loop {
        let Ok((stream, _)) = listener.accept().await else {
            continue;
        };
        let (token, sink) = (token.clone(), sink.clone());
        tokio::spawn(async move {
            let service = service_fn(move |request| {
                let (token, sink) = (token.clone(), sink.clone());
                async move { Ok::<_, Infallible>(route(request, &token, &sink).await) }
            });
            let _ = http1::Builder::new()
                .serve_connection(TokioIo::new(stream), service)
                .await;
        });
    }
}

/// The only route there is.
async fn route(request: Request<Incoming>, token: &str, sink: &Sink) -> Response<Full<Bytes>> {
    if !bearer(&request, token) {
        return reply(StatusCode::UNAUTHORIZED);
    }
    if request.method() != Method::POST {
        return reply(StatusCode::METHOD_NOT_ALLOWED);
    }
    let Some(session) = session_of(request.uri().path()) else {
        return reply(StatusCode::NOT_FOUND);
    };
    let Ok(body) = Limited::new(request.into_body(), BODY_MAX).collect().await else {
        return reply(StatusCode::PAYLOAD_TOO_LARGE);
    };
    match parse::post(&session, &body.to_bytes()) {
        Some(post) => {
            sink(post);
            reply(StatusCode::NO_CONTENT)
        }
        None => reply(StatusCode::BAD_REQUEST),
    }
}

fn bearer(request: &Request<Incoming>, token: &str) -> bool {
    request
        .headers()
        .get(hyper::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .is_some_and(|value| value == token)
}

/// `/hook/<session>`, and nothing shorter or longer.
fn session_of(path: &str) -> Option<String> {
    let rest = path.strip_prefix("/hook/")?;
    let session = rest.trim_end_matches('/');
    (!session.is_empty() && !session.contains('/')).then(|| session.to_string())
}

fn reply(status: StatusCode) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .body(Full::new(Bytes::new()))
        .unwrap_or_default()
}
