//! A local HTTP server: a free port on 127.0.0.1, a bearer token, one task a connection.

use std::convert::Infallible;
use std::net::{Ipv4Addr, SocketAddr};

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

/// The port the server took, and the token every request must carry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bound {
    pub port: u16,
    pub token: String,
}

/// Takes a free loopback port and serves `route` on it until the process ends.
/// A request without the token is refused before `route` sees it.
pub fn serve<R, F, B>(handle: &Handle, route: R) -> std::io::Result<Bound>
where
    R: Fn(Request<Incoming>) -> F + Clone + Send + Sync + 'static,
    F: Future<Output = Response<B>> + Send + 'static,
    B: Body<Data = Bytes> + Default + Send + 'static,
    B::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
    let listener = handle.block_on(TcpListener::bind(address))?;
    let port = listener.local_addr()?.port();
    let token = uuid::Uuid::new_v4().simple().to_string();
    handle.spawn(accept(listener, token.clone(), route));
    Ok(Bound { port, token })
}

/// One connection at a time off the socket, each served on its own task.
async fn accept<R, F, B>(listener: TcpListener, token: String, route: R)
where
    R: Fn(Request<Incoming>) -> F + Clone + Send + Sync + 'static,
    F: Future<Output = Response<B>> + Send + 'static,
    B: Body<Data = Bytes> + Default + Send + 'static,
    B::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    loop {
        let Ok((stream, _)) = listener.accept().await else {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            continue;
        };
        let (token, route) = (token.clone(), route.clone());
        tokio::spawn(async move {
            let service = service_fn(move |request| {
                let (token, route) = (token.clone(), route.clone());
                async move {
                    Ok::<_, Infallible>(match bearer(&request, &token) {
                        true => route(request).await,
                        false => reply(StatusCode::UNAUTHORIZED),
                    })
                }
            });
            let _ = http1::Builder::new()
                .serve_connection(TokioIo::new(stream), service)
                .await;
        });
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
