//! The cluster part of the app: the contexts found, their sign-in, and the watched objects.

mod store;
#[cfg(test)]
mod tests;

use std::path::PathBuf;

use groove_types::{KubeContext, KubeKind, Login, Result, WatchKey};

pub use groove_contexts::paths;
pub use groove_objects::{Batch, Delta, Stop};
pub use store::{Store, Watched};

/// The `cluster` slice of `AppState`.
#[derive(Debug, Default)]
pub struct State {
    /// What the last scan found; `None` before the first one ends.
    pub found: Option<Vec<KubeContext>>,
    pub scanning: bool,
    logins: Vec<(String, Login)>,
    checking: Vec<String>,
    pub store: Store,
}

impl State {
    /// What the last check of `context` found.
    pub fn login(&self, context: &str) -> Option<&Login> {
        let held = self.logins.iter().find(|(name, _)| name == context);
        held.map(|(_, login)| login)
    }

    pub fn checking(&self, context: &str) -> bool {
        self.checking.iter().any(|name| name == context)
    }

    /// Marks a scan begun. False while one already runs.
    pub fn begin_scan(&mut self) -> bool {
        !std::mem::replace(&mut self.scanning, true)
    }

    /// Marks a check of `context` begun. False while one already runs.
    pub fn begin_check(&mut self, context: &str) -> bool {
        if self.checking(context) {
            return false;
        }
        self.checking.push(context.to_string());
        true
    }
}

#[derive(Debug)]
pub enum Event {
    Found(Vec<KubeContext>),
    /// The scan could not read the files; what it found before stays.
    Unread,
    Checked {
        context: String,
        login: Login,
    },
    Kinds {
        context: String,
        kinds: Vec<KubeKind>,
    },
    Watched {
        key: WatchKey,
        batch: Batch,
    },
}

pub fn apply(state: &mut State, event: Event) {
    match event {
        Event::Found(contexts) => {
            state.found = Some(contexts);
            state.scanning = false;
        }
        Event::Unread => state.scanning = false,
        Event::Checked { context, login } => {
            state.checking.retain(|name| *name != context);
            state.logins.retain(|(name, _)| *name != context);
            state.logins.push((context, login));
        }
        Event::Kinds { context, kinds } => state.store.set_kinds(&context, kinds),
        Event::Watched { key, batch } => state.store.apply(&key, batch),
    }
}

/// Every context the files name.
pub async fn scan(paths: Vec<PathBuf>) -> Result<Vec<KubeContext>> {
    groove_contexts::found(&paths).await
}

/// The kinds of `context`, from the cache under `cache` while it is fresh unless `again`.
pub async fn kinds(
    paths: Vec<PathBuf>,
    context: String,
    cache: PathBuf,
    again: bool,
) -> Result<Vec<KubeKind>> {
    groove_objects::kinds(&paths, &context, &cache, again).await
}

/// The watcher of `key`, until `stop`; each batch goes through `send`.
pub fn watcher(
    paths: Vec<PathBuf>,
    key: WatchKey,
    stop: &Stop,
    send: impl Fn(Batch) + Send + Sync + 'static,
) -> impl std::future::Future<Output = ()> + Send + 'static {
    groove_objects::watch(paths, key, stop, send)
}

/// Whether `context` signs in now.
pub async fn check(paths: Vec<PathBuf>, context: String) -> Login {
    groove_contexts::check(&paths, &context).await
}
