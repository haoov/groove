use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use crate::AppState;

/// A result on its way back to the main thread: it writes into `AppState` there.
pub type Continuation = Box<dyn FnOnce(&mut AppState) + Send>;

/// The async part of a controller function, ending in its continuation.
pub type Job = Pin<Box<dyn Future<Output = Continuation> + Send>>;

/// Hands a continuation to the main thread. The binary implements it over winit's proxy.
pub trait Deliver: Send + Sync {
    fn deliver(&self, continuation: Continuation);
}

pub trait Spawner {
    fn spawn(&self, job: Job);
}

/// Runs jobs on the tokio pool and delivers their continuations.
pub struct TokioSpawner {
    runtime: tokio::runtime::Handle,
    sink: Arc<dyn Deliver>,
}

impl TokioSpawner {
    pub fn new(runtime: tokio::runtime::Handle, sink: Arc<dyn Deliver>) -> Self {
        Self { runtime, sink }
    }
}

impl Spawner for TokioSpawner {
    fn spawn(&self, job: Job) {
        let sink = self.sink.clone();
        self.runtime.spawn(async move { sink.deliver(job.await) });
    }
}

/// Runs each job to completion on the spot and keeps its continuation for `drain`.
pub struct SyncSpawner {
    runtime: tokio::runtime::Runtime,
    pending: Mutex<Vec<Continuation>>,
}

impl SyncSpawner {
    pub fn new() -> std::io::Result<Self> {
        Ok(Self {
            runtime: tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?,
            pending: Mutex::new(Vec::new()),
        })
    }

    /// Applies every finished continuation, in order.
    pub fn drain(&self, state: &mut AppState) {
        let pending = std::mem::take(&mut *self.pending.lock().unwrap_or_else(|e| e.into_inner()));
        for continuation in pending {
            continuation(state);
        }
    }
}

impl Spawner for SyncSpawner {
    fn spawn(&self, job: Job) {
        let continuation = self.runtime.block_on(job);
        self.pending
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(continuation);
    }
}
