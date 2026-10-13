//! A local HTTP server: a free port on 127.0.0.1, a bearer token per session, one task a connection.

use std::collections::HashMap;
use std::convert::Infallible;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex, PoisonError};

use http_body_util::{BodyExt, Limited};
use hyper::body::{Body, Bytes, Incoming};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use tokio::runtime::Handle;

/// The most a request body may carry.
pub const BODY_MAX: usize = 1 << 20;

/// The port the server took, and the tokens its requests carry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bound {
    pub port: u16,
    pub keys: Keys,
}

/// One token per session, made the first time it is asked for.
#[derive(Debug, Clone, Default)]
pub struct Keys(Arc<Mutex<HashMap<String, String>>>);

impl PartialEq for Keys {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for Keys {}

impl Keys {
    pub fn of(&self, session: &str) -> String {
        let mut held = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        let made = || uuid::Uuid::new_v4().simple().to_string();
        held.entry(session.to_string()).or_insert_with(made).clone()
    }

    pub fn holder(&self, token: &str) -> Option<String> {
        let held = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        let mut found = None;
        for (session, own) in held.iter() {
            if same(own.as_bytes(), token.as_bytes()) {
                found = Some(session.clone());
            }
        }
        found
    }
}

/// Equal bytes, compared in a time that does not depend on where they differ.
fn same(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Takes a free loopback port and serves `route` on it until the process ends.
/// A request without a session's token is refused; `route` gets the session that holds it.
pub fn serve<R, F, B>(handle: &Handle, route: R) -> std::io::Result<Bound>
where
    R: Fn(Request<Incoming>, String) -> F + Clone + Send + Sync + 'static,
    F: Future<Output = Response<B>> + Send + 'static,
    B: Body<Data = Bytes> + Default + Send + 'static,
    B::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
    let listener = handle.block_on(TcpListener::bind(address))?;
    let port = listener.local_addr()?.port();
    let keys = Keys::default();
    handle.spawn(accept(listener, keys.clone(), route));
    Ok(Bound { port, keys })
}

/// One connection at a time off the socket, each served on its own task.
async fn accept<R, F, B>(listener: TcpListener, keys: Keys, route: R)
where
    R: Fn(Request<Incoming>, String) -> F + Clone + Send + Sync + 'static,
    F: Future<Output = Response<B>> + Send + 'static,
    B: Body<Data = Bytes> + Default + Send + 'static,
    B::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    loop {
        let Ok((stream, _)) = listener.accept().await else {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            continue;
        };
        let (keys, route) = (keys.clone(), route.clone());
        tokio::spawn(async move {
            let service = service_fn(move |request| {
                let (keys, route) = (keys.clone(), route.clone());
                async move {
                    Ok::<_, Infallible>(match bearer(&request).and_then(|one| keys.holder(one)) {
                        Some(session) => route(request, session).await,
                        None => reply(StatusCode::UNAUTHORIZED),
                    })
                }
            });
            let _ = http1::Builder::new()
                .serve_connection(TokioIo::new(stream), service)
                .await;
        });
    }
}

fn bearer(request: &Request<Incoming>) -> Option<&str> {
    request
        .headers()
        .get(hyper::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
}

/// The request's body, or nothing when it runs past `BODY_MAX` or breaks off.
pub async fn body(request: Request<Incoming>) -> Option<Bytes> {
    let read = Limited::new(request.into_body(), BODY_MAX).collect().await;
    read.ok().map(|whole| whole.to_bytes())
}

/// A status with nothing under it.
pub fn reply<B: Default>(status: StatusCode) -> Response<B> {
    let mut response = Response::new(B::default());
    *response.status_mut() = status;
    response
}
