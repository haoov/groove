//! Messages out, and each request's answer matched back by its id.

use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use groove_types::{Error, ErrorKind, Result};
use serde_json::{Value, json};
use tokio::sync::{mpsc, oneshot};

/// How long a request waits for its answer.
pub(crate) const WAIT: Duration = Duration::from_secs(10);

/// A result, or the message of the error the server answered.
pub(crate) type Answer = std::result::Result<Value, String>;

pub(crate) struct Link {
    outgoing: mpsc::UnboundedSender<Value>,
    pending: Mutex<HashMap<i64, oneshot::Sender<Answer>>>,
    next: AtomicI64,
}

impl Link {
    pub(crate) fn new(outgoing: mpsc::UnboundedSender<Value>) -> Self {
        Self {
            outgoing,
            pending: Mutex::new(HashMap::new()),
            next: AtomicI64::new(1),
        }
    }

    pub(crate) async fn request(&self, method: &str, params: Value) -> Result<Value> {
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let (sender, answer) = oneshot::channel();
        lock(&self.pending).insert(id, sender);
        self.send(json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }))?;
        match tokio::time::timeout(WAIT, answer).await {
            Ok(Ok(Ok(result))) => Ok(result),
            Ok(Ok(Err(message))) => Err(Error::invalid(format!("{method}: {message}"))),
            Ok(Err(_)) => Err(closed()),
            Err(_) => {
                lock(&self.pending).remove(&id);
                Err(Error::new(
                    ErrorKind::Io,
                    format!(
                        "the language server did not answer {method} in {}s",
                        WAIT.as_secs()
                    ),
                ))
            }
        }
    }

    pub(crate) fn notify(&self, method: &str, params: Value) -> Result<()> {
        self.send(json!({ "jsonrpc": "2.0", "method": method, "params": params }))
    }

    pub(crate) fn reply(&self, id: Value, result: Value) -> Result<()> {
        self.send(json!({ "jsonrpc": "2.0", "id": id, "result": result }))
    }

    pub(crate) fn refuse(&self, id: Value, code: i64, message: &str) -> Result<()> {
        let error = json!({ "code": code, "message": message });
        self.send(json!({ "jsonrpc": "2.0", "id": id, "error": error }))
    }

    /// Hands `answer` to the request `id` waits on, if it still waits.
    pub(crate) fn settle(&self, id: i64, answer: Answer) {
        if let Some(sender) = lock(&self.pending).remove(&id) {
            let _ = sender.send(answer);
        }
    }

    /// Fails every request still waiting: the server is gone.
    pub(crate) fn close(&self) {
        lock(&self.pending).clear();
    }

    fn send(&self, message: Value) -> Result<()> {
        self.outgoing.send(message).map_err(|_| closed())
    }
}

pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn closed() -> Error {
    Error::new(ErrorKind::Io, "the language server has closed")
}
