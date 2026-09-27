//! One event stream a connection, and the table that finds it again.

use std::collections::HashMap;
use std::convert::Infallible;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Duration;

use http_body_util::combinators::BoxBody;
use hyper::body::{Bytes, Frame};
use hyper::{Response, StatusCode};
use tokio::sync::mpsc;

pub type Body = BoxBody<Bytes, Infallible>;

const DEPTH: usize = 64;
const ALIVE_EVERY: Duration = Duration::from_secs(15);

/// The live streams, by the connection id their messages carry.
#[derive(Clone, Default)]
pub struct Connections(Arc<Mutex<HashMap<String, Open>>>);

/// One stream, and the session whose agent holds it.
#[derive(Clone)]
pub struct Open {
    pub session: String,
    tx: mpsc::Sender<Bytes>,
}

impl Open {
    /// One JSON-RPC answer, back where it was asked.
    pub async fn send(&self, message: &serde_json::Value) {
        let _ = self.tx.send(event("message", &message.to_string())).await;
    }
}

impl Connections {
    pub fn open(&self, id: &str) -> Option<Open> {
        self.0.lock().ok()?.get(id).cloned()
    }

    fn hold(&self, id: &str, open: Open) {
        if let Ok(mut live) = self.0.lock() {
            live.insert(id.to_string(), open);
        }
    }

    fn forget(&self, id: &str) {
        if let Ok(mut live) = self.0.lock() {
            live.remove(id);
        }
    }
}

/// The stream itself: the first event says where to post, the rest are answers.
pub fn open(live: &Connections, query: Option<&str>) -> Response<Body> {
    let Some(session) = param(query, "task") else {
        return reply(StatusCode::BAD_REQUEST);
    };
    let id = uuid::Uuid::new_v4().simple().to_string();
    let (tx, rx) = mpsc::channel(DEPTH);
    let _ = tx.try_send(event("endpoint", &format!("/message?sessionId={id}")));
    live.hold(
        &id,
        Open {
            session,
            tx: tx.clone(),
        },
    );
    tokio::spawn(alive(tx));
    let body = Sse {
        rx,
        live: live.clone(),
        id,
    };
    Response::builder()
        .status(StatusCode::OK)
        .header(hyper::header::CONTENT_TYPE, "text/event-stream")
        .header(hyper::header::CACHE_CONTROL, "no-cache")
        .body(BoxBody::new(body))
        .unwrap_or_else(|_| reply(StatusCode::INTERNAL_SERVER_ERROR))
}

/// A comment on the stream every `ALIVE_EVERY`.
async fn alive(tx: mpsc::Sender<Bytes>) {
    loop {
        tokio::time::sleep(ALIVE_EVERY).await;
        if tx.send(Bytes::from_static(b": alive\n\n")).await.is_err() {
            return;
        }
    }
}

/// The body of a stream, which forgets its connection when the agent drops it.
struct Sse {
    rx: mpsc::Receiver<Bytes>,
    live: Connections,
    id: String,
}

impl hyper::body::Body for Sse {
    type Data = Bytes;
    type Error = Infallible;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, Infallible>>> {
        self.rx
            .poll_recv(cx)
            .map(|one| one.map(|bytes| Ok(Frame::data(bytes))))
    }
}

impl Drop for Sse {
    fn drop(&mut self) {
        self.live.forget(&self.id);
    }
}

fn event(kind: &str, data: &str) -> Bytes {
    Bytes::from(format!("event: {kind}\ndata: {data}\n\n"))
}

/// One query argument, from a query string that may hold several.
pub fn param(query: Option<&str>, name: &str) -> Option<String> {
    query?
        .split('&')
        .filter_map(|one| one.split_once('='))
        .find(|(key, value)| *key == name && !value.is_empty())
        .map(|(_, value)| value.to_string())
}

pub fn reply(status: StatusCode) -> Response<Body> {
    Response::builder()
        .status(status)
        .body(BoxBody::new(http_body_util::Empty::new()))
        .unwrap_or_default()
}
