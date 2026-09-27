//! How a job leaves the main thread and how its result comes back.

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use groove_types::Error;

use crate::{AppState, Services};

/// A result on its way back to the main thread. It writes into `AppState` there, and
/// may start the next step of its controller function.
pub type Continuation = Box<dyn FnOnce(&mut AppState, &Services, &dyn Spawner) + Send>;

/// The async part of a controller function, ending in its continuation.
pub type Job = Pin<Box<dyn Future<Output = Continuation> + Send>>;

/// Hands a continuation to the main thread. The binary implements it over winit's proxy.
pub trait Deliver: Send + Sync {
    fn deliver(&self, continuation: Continuation);
}

pub trait Spawner {
    fn spawn(&self, job: Job);

    /// The sink a thread reports back through.
    fn sink(&self) -> Arc<dyn Deliver>;
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

    fn sink(&self) -> Arc<dyn Deliver> {
        self.sink.clone()
    }
}

/// Continuations waiting for `drain`.
#[derive(Default)]
struct Pending(Mutex<Vec<Continuation>>);

impl Deliver for Pending {
    fn deliver(&self, continuation: Continuation) {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(continuation);
    }
}

/// Runs each job to completion on the spot and keeps its continuation for `drain`.
pub struct SyncSpawner {
    runtime: tokio::runtime::Runtime,
    pending: Arc<Pending>,
}

impl SyncSpawner {
    pub fn new() -> std::io::Result<Self> {
        Ok(Self {
            runtime: tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?,
            pending: Arc::default(),
        })
    }

    /// Applies every continuation that has arrived, in order.
    pub fn drain(&self, state: &mut AppState, services: &Services) {
        let pending = std::mem::take(
            &mut *self
                .pending
                .0
                .lock()
                .unwrap_or_else(PoisonError::into_inner),
        );
        for continuation in pending {
            continuation(state, services, self);
        }
    }

    /// Runs a future to completion on the spawner's own runtime.
    pub fn block_on<F: Future>(&self, future: F) -> F::Output {
        self.runtime.block_on(future)
    }
}

impl Spawner for SyncSpawner {
    fn spawn(&self, job: Job) {
        let continuation = self.runtime.block_on(job);
        self.pending.deliver(continuation);
    }

    fn sink(&self) -> Arc<dyn Deliver> {
        self.pending.clone()
    }
}

/// A callback that delivers at most one continuation until that one has run.
pub fn coalesced(
    sink: Arc<dyn Deliver>,
    make: impl Fn() -> Continuation + Send + Sync + 'static,
) -> impl Fn() + Send + Sync {
    let in_flight = Arc::new(AtomicBool::new(false));
    move || {
        if in_flight.swap(true, Ordering::AcqRel) {
            return;
        }
        let flag = in_flight.clone();
        let continuation = make();
        sink.deliver(Box::new(
            move |state: &mut AppState, services: &Services, spawner: &dyn Spawner| {
                flag.store(false, Ordering::Release);
                continuation(state, services, spawner);
            },
        ));
    }
}

/// A write whose only result is success or an error for the feed.
pub(crate) fn record(
    spawner: &dyn Spawner,
    write: impl Future<Output = Result<(), Error>> + Send + 'static,
) {
    spawner.spawn(Box::pin(async move {
        let result = write.await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            if let Err(e) = result {
                state.failed(e);
            }
        }) as Continuation
    }));
}
